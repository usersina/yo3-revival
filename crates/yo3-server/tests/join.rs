//! The join page (`/join`) of the patch HTTP server, through the real server.
use std::net::IpAddr;
use std::time::Instant;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use yo3_server::accounts::{Directory, JOIN_PER_ADDRESS, JOIN_PER_HOUR};
use yo3_server::{Config, Running, start};

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("yo3-join-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

async fn server(dir: &std::path::Path, join: bool) -> Running {
    start(Config { data_dir: Some(dir.to_path_buf()), join, ..Config::ephemeral(dir.join("logs")) }).await.unwrap()
}

/// A GET with a `Host` header: the whole response (head and body).
async fn get(server: &Running, path: &str, host: &str) -> String {
    let mut s = TcpStream::connect(server.http).await.unwrap();
    s.write_all(format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes()).await.unwrap();
    let mut resp = String::new();
    s.read_to_string(&mut resp).await.unwrap();
    resp
}

fn key_in(resp: &str) -> String {
    let at = resp.find("/yo3/").expect("a patch url") + 5;
    resp[at..].split('/').next().unwrap().to_string()
}

fn accounts(dir: &std::path::Path) -> Directory {
    Directory::open(vec!["Duelist".into()], &dir.join("accounts.json")).unwrap()
}

#[tokio::test]
async fn the_form_is_served() {
    let dir = scratch("form");
    let s = server(&dir, true).await;
    let r = get(&s, "/join", "play.example:8080").await;
    assert!(r.starts_with("HTTP/1.1 200") && r.contains("text/html") && r.contains("<form") && r.contains("name=\"name\""), "{r}");
    assert!(r.contains("secret") && r.contains("VPN"), "the key warning");
    assert!(!r.contains("[Yo3]"), "no account yet");
}

#[tokio::test]
async fn a_join_makes_a_key_account_and_shows_the_lines() {
    let dir = scratch("make");
    let s = server(&dir, true).await;
    let r = get(&s, "/join?name=Yugi+Muto", "play.example:8080").await;
    assert!(r.starts_with("HTTP/1.1 200"), "{r}");
    let key = key_in(&r);
    let http = s.http.port();
    assert!(r.contains("GateServerAddress=play.example\n"), "host without its port: {r}");
    assert!(r.contains(&format!("GateServerPort={}\n", s.gate.port())));
    assert!(r.contains(&format!("PatchUpdateUrl=http://play.example:{http}/yo3/{key}/")));
    assert!(r.contains(&format!("PatchDataUrl=http://play.example:{http}/ygoda/patch/")));
    assert!(r.contains(&format!("InfoURL=http://play.example:{http}/app/")));
    assert!(r.contains(&format!("/join/yo3.ini?key={key}")) && r.contains("Program Files (x86)"), "download link and steps");
    // the account exists with that key and player
    let d = accounts(&dir);
    let (_, k, acc) = d.entries().next().unwrap();
    assert_eq!(k, Some(key.as_str()));
    assert_eq!(acc.players.len(), 1);
    assert_eq!(acc.players[0].name, "Yugi Muto");
}

#[tokio::test]
async fn the_download_is_a_crlf_ini_attachment() {
    let dir = scratch("ini");
    let s = server(&dir, true).await;
    let key = key_in(&get(&s, "/join?name=Kaiba", "10.0.0.5:8080").await);
    let r = get(&s, &format!("/join/yo3.ini?key={key}"), "10.0.0.5:8080").await;
    assert!(r.starts_with("HTTP/1.1 200"), "{r}");
    assert!(r.contains("Content-Disposition: attachment; filename=\"yo3.ini\""), "{r}");
    let body = r.split("\r\n\r\n").nth(1).unwrap();
    let http = s.http.port();
    let want = format!(
        "[Yo3]\r\nGateServerAddress=10.0.0.5\r\nGateServerPort={}\r\nPatchUpdateUrl=http://10.0.0.5:{http}/yo3/{key}/\r\nPatchDataUrl=http://10.0.0.5:{http}/ygoda/patch/\r\nInfoURL=http://10.0.0.5:{http}/app/\r\n",
        s.gate.port()
    );
    assert_eq!(body, want);
    assert!(get(&s, "/join/yo3.ini?key=nope", "x").await.starts_with("HTTP/1.1 404"), "an unknown key has no file");
}

#[tokio::test]
async fn a_bad_or_taken_name_is_refused_with_a_message() {
    let dir = scratch("names");
    let s = server(&dir, true).await;
    assert!(get(&s, "/join?name=Yugi", "h").await.starts_with("HTTP/1.1 200"));
    let taken = get(&s, "/join?name=yUGI", "h").await;
    assert!(taken.starts_with("HTTP/1.1 400") && taken.contains("is taken"), "{taken}");
    let long = get(&s, "/join?name=ABCDEFGHIJKLMNOPQ", "h").await;
    assert!(long.starts_with("HTTP/1.1 400") && long.contains("1 to 16 characters"), "{long}");
    for bad in ["", "%20%20", "A%07B"] {
        let r = get(&s, &format!("/join?name={bad}"), "h").await;
        assert!(r.starts_with("HTTP/1.1 400") && r.contains("class=\"err\""), "{bad:?}: {r}");
    }
    assert_eq!(accounts(&dir).entries().count(), 1, "nothing else was made");
}

#[tokio::test]
async fn everything_shown_is_escaped() {
    let dir = scratch("escape");
    let s = server(&dir, true).await;
    let r = get(&s, "/join?name=%3Cb%3E%22x%26", "h").await;
    assert!(r.contains("&lt;b&gt;&quot;x&amp;") && !r.contains("<b>"), "{r}");
    let r = get(&s, "/join?name=a", "evil<script>").await;
    assert!(!r.contains("<script>") && r.contains("GateServerAddress=HOST"), "a strange Host is not echoed: {r}");
}

#[tokio::test]
async fn a_repeat_visit_gets_the_same_key_back() {
    let dir = scratch("repeat");
    let s = server(&dir, true).await;
    let first = key_in(&get(&s, "/join?name=Joey", "h:8080").await);
    let again = get(&s, "/join", "h:8080").await;
    assert_eq!(key_in(&again), first);
    assert!(again.contains("already has an account"), "{again}");
    assert_eq!(accounts(&dir).entries().count(), 1);
}

#[tokio::test]
async fn an_address_only_account_can_be_kept_with_a_key() {
    let dir = scratch("keep");
    // A friend who played before keys: an account found by the address alone, with a first player.
    let loopback: IpAddr = [127, 0, 0, 1].into();
    let before = accounts(&dir).account_for(loopback);
    let s = server(&dir, true).await;
    let page = get(&s, "/join", "h:8080").await;
    assert!(page.contains("Keep my account") && !page.contains("[Yo3]"), "{page}");
    let kept = get(&s, "/join?keep=1", "h:8080").await;
    let key = key_in(&kept);
    let d = accounts(&dir);
    let (addr, k, acc) = d.entries().next().unwrap();
    assert_eq!((addr, k, acc.user_id), (Some(loopback), Some(key.as_str()), before.user_id), "same account, now with a key");
    assert_eq!(d.entries().count(), 1);
    // a second keep gives the same key
    assert_eq!(key_in(&get(&s, "/join?keep=1", "h:8080").await), key);
}

#[tokio::test]
async fn an_address_makes_a_few_accounts_and_the_page_makes_a_few_per_hour() {
    let dir = scratch("cap");
    let s = server(&dir, true).await;
    for i in 0..JOIN_PER_ADDRESS {
        assert!(get(&s, &format!("/join?name=P{i}"), "h").await.starts_with("HTTP/1.1 200"));
    }
    let r = get(&s, "/join?name=OneMore", "h").await;
    assert!(r.starts_with("HTTP/1.1 400") && r.contains("already made"), "{r}");
    assert_eq!(accounts(&dir).entries().count(), JOIN_PER_ADDRESS);
    // the hourly cap, over many addresses
    let mut d = Directory::new(vec!["Duelist".into()]);
    let now = Instant::now();
    for i in 0..JOIN_PER_HOUR {
        let addr: IpAddr = [10, 0, 0, i as u8].into();
        d.join(addr, &format!("Q{i}"), now).unwrap();
    }
    assert!(d.join([10, 0, 1, 1].into(), "Late", now).unwrap_err().contains("last hour"));
    assert!(d.join([10, 0, 1, 1].into(), "Late", now + std::time::Duration::from_secs(3601)).is_ok());
}

#[tokio::test]
async fn no_join_turns_the_page_off() {
    let dir = scratch("off");
    let s = server(&dir, false).await;
    assert!(get(&s, "/join", "h").await.starts_with("HTTP/1.1 404"));
    assert!(get(&s, "/join?name=Yugi", "h").await.starts_with("HTTP/1.1 404"));
    assert!(!dir.join("accounts.json").exists());
}

#[tokio::test]
async fn a_joined_key_binds_the_account_like_an_added_one() {
    let dir = scratch("bind");
    let s = server(&dir, true).await;
    let key = key_in(&get(&s, "/join?name=Seto", "h:8080").await);
    let _ = get(&s, &format!("/yo3/{key}/yo3.idx"), "h:8080").await;
    // the next front login from this address lands in the key's account: its only player is Seto
    use yo3_proto::{Message, Packet, Value, decrypt, frame, frame_len};
    let mut front = TcpStream::connect(s.front).await.unwrap();
    front.write_all(&frame(&[b"\x00\x122010.03.23 skelton".as_slice(), &[0; 4]].concat())).await.unwrap();
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
    recv(&mut front).await;
    front.write_all(&frame(&Message::reply(0x1F49).encode())).await.unwrap();
    let list = recv(&mut front).await;
    let names: Vec<_> = list.items.iter().map(|i| i.get("pname").and_then(Value::as_str).unwrap_or_default()).collect();
    assert_eq!(names, ["Seto"]);
}

#[tokio::test]
async fn the_page_warns_when_duels_are_unavailable() {
    let dir = scratch("noengine");
    let s = start(Config {
        data_dir: Some(dir.to_path_buf()),
        engine_problem: Some("Node.js v20.0.0 is too old".into()),
        ..Config::ephemeral(dir.join("logs"))
    })
    .await
    .unwrap();
    let r = get(&s, "/join", "play.example:8080").await;
    assert!(r.contains("Duels are unavailable on this server") && r.contains("Node.js 22.18"), "{r}");
    // A server whose engine can run says nothing of it.
    let dir = scratch("engine-ok");
    let ok = server(&dir, true).await;
    let r = get(&ok, "/join", "play.example:8080").await;
    assert!(r.contains("<form"), "the form is still served");
}
