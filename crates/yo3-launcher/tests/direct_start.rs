//! Direct start's sign-in (#33): each row of the issue's table, as far as the helper decides it. The loader's and the
//! game's side of the same rows run under Wine in tools/mod/test-direct.sh.
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use yo3_launcher::direct::{self, Refusal};
use yo3_launcher::vault::{MemoryVault, Vault};
use yo3_launcher::{Credential, Ports, Proxy};
use yo3_proto::{Message, Packet, decrypt, frame, frame_len};
use yo3_tunnel::Endpoint;

async fn read(s: &mut TcpStream) -> Message {
    let mut h = [0; 4]; s.read_exact(&mut h).await.unwrap();
    let mut body = vec![0; frame_len(h)]; s.read_exact(&mut body).await.unwrap();
    let Packet::Message(m) = Packet::decode(&decrypt(&body).unwrap()).unwrap() else { panic!("message") }; m
}
async fn local(p: &Proxy, port: u16) -> TcpStream {
    let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    s.write_all(&p.cookie).await.unwrap(); s
}
const OPENER: &[u8] = b"\x00\x122010.03.23 skelton";

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("yo3-direct-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir); dir
}

/// What the game sees when the helper could not sign in: its own login dialog, reached through the usual gate and
/// front, never a dropped connection (which would end in a foreign "no response" or a crash).
#[tokio::test]
async fn a_refused_sign_in_reaches_the_game_as_its_own_login_dialog() {
    let dir = scratch("refused");
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    for (refusal, reply) in [(Refusal::FailedToConnect, 0x21A0), (Refusal::NeedsUpdate, 0x21A0), (Refusal::SignedOut, 0x21A0),
        (Refusal::WrongPassword, 0x219B), (Refusal::Restricted, 0x219E)] {
        let endpoint = Endpoint::new("127.0.0.1:1".into(), "localhost".into(), &cfg.tls.certificate).unwrap();
        let proxy = Proxy::prepare(endpoint, Ports::default()).await.unwrap();
        let mut gate = local(&proxy, proxy.ports.gate).await;
        gate.write_all(&frame(OPENER)).await.unwrap();
        assert_eq!(read(&mut gate).await.request(), Some(0x21FE), "the gate redirects as usual");
        let mut front = local(&proxy, proxy.ports.front).await;
        front.write_all(&frame(OPENER)).await.unwrap();
        // The game's ONLINE may come before the helper knows: the front waits for the outcome.
        tokio::time::sleep(Duration::from_millis(50)).await;
        proxy.refuse(refusal);
        assert_eq!(read(&mut front).await.request(), Some(0x219F), "{refusal:?}");
        front.write_all(&frame(&Message::reply(0x1F47).encode())).await.unwrap();
        assert_eq!(read(&mut front).await.request(), Some(reply), "{refusal:?}");
        assert!(tokio::time::timeout(Duration::from_millis(75), front.read(&mut [0; 1])).await.is_err(),
            "the front stays open until the game closes it");
        let mut duel = local(&proxy, proxy.ports.duel).await;
        assert_eq!(tokio::time::timeout(Duration::from_secs(1), duel.read(&mut [0; 1])).await.unwrap().unwrap_or(0), 0);
    }
    assert_eq!(Refusal::NeedsUpdate.notice(), Some(direct::UPDATE_NOTICE));
    assert_eq!(Refusal::FailedToConnect.notice(), None, "a server that is down needs no line of ours");
    for (code, refusal) in [("invalid_credentials", Refusal::WrongPassword), ("banned", Refusal::Restricted), ("restricted", Refusal::Restricted),
        ("update_required", Refusal::NeedsUpdate), ("signed_out", Refusal::SignedOut), ("maintenance", Refusal::FailedToConnect),
        ("server_full", Refusal::FailedToConnect), ("throttled", Refusal::FailedToConnect)] {
        assert_eq!(Refusal::from_account_error(code), refusal, "{code}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// Multiplayer rows: a valid saved sign-in plays; none, or one revoked by Delete Login Data, is "signed out"; a server
/// that is down is "Failed to connect". Delete Login Data's request reaches the helper through the HTTP port.
#[tokio::test]
async fn a_saved_sign_in_resumes_and_delete_login_data_revokes_it() {
    yo3_server::logins::use_fast_hashing();
    let dir = scratch("device");
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    let cert = cfg.tls.certificate.clone();
    let server = yo3_server::start(cfg).await.unwrap();
    let endpoint = Endpoint::new(server.auth.to_string(), "localhost".into(), &cert).unwrap();
    let made = endpoint.account(&serde_json::json!({ "op": "register", "user": "direct", "password": "a long passphrase", "rules": true })).await.unwrap();
    assert_eq!(made["ok"], true, "{made}");
    let signed = endpoint.account(&serde_json::json!({ "op": "login", "user": "direct", "password": "a long passphrase", "stay": true, "device": "test" })).await.unwrap();
    let token = signed["device_token"].as_str().unwrap().to_string();
    let vault = MemoryVault::default();
    let account = "server:test";
    let read = |a: &str| vault.get(a).unwrap();
    let device = Credential::Device(account.into());
    let failure = direct::sign_in(&endpoint, &device, &read).await.unwrap_err();
    assert_eq!(failure.refusal, Refusal::SignedOut, "no saved sign-in: {}", failure.reason);
    vault.set(account, &token).unwrap();
    direct::sign_in(&endpoint, &device, &read).await.unwrap();
    // Delete Login Data, as the DLL asks it.
    let proxy = Proxy::prepare(endpoint.clone(), Ports::default()).await.unwrap();
    let asked = proxy.sign_outs();
    let waiting = tokio::spawn(async move { asked.notified().await });
    let mut http = TcpStream::connect(("127.0.0.1", proxy.ports.http)).await.unwrap();
    http.write_all(format!("GET /session/{}/{} HTTP/1.1\r\nHost: localhost\r\n\r\n", yo3_launcher::hex(&proxy.cookie), yo3_launcher::SIGN_OUT_PATH).as_bytes()).await.unwrap();
    let mut reply = vec![]; http.read_to_end(&mut reply).await.unwrap();
    assert!(reply.starts_with(b"HTTP/1.1 204"), "{}", String::from_utf8_lossy(&reply));
    tokio::time::timeout(Duration::from_secs(1), waiting).await.unwrap().unwrap();
    assert!(direct::sign_out(&endpoint, Some(account), &vault).await.unwrap());
    assert_eq!(vault.get(account).unwrap(), None, "the device forgets its sign-in");
    vault.set(account, &token).unwrap();
    let failure = direct::sign_in(&endpoint, &device, &read).await.unwrap_err();
    assert_eq!(failure.refusal, Refusal::SignedOut, "the server revoked it: {}", failure.reason);
    assert!(!direct::sign_out(&endpoint, None, &vault).await.unwrap(), "Singleplayer has nothing to delete");
    let down = Endpoint::new("127.0.0.1:1".into(), "localhost".into(), &cert).unwrap();
    assert_eq!(direct::sign_in(&down, &device, &read).await.unwrap_err().refusal, Refusal::FailedToConnect);
    assert_eq!(direct::sign_in(&down, &Credential::Key("k".into()), &read).await.unwrap_err().refusal, Refusal::FailedToConnect);
    let _ = std::fs::remove_dir_all(dir);
}

/// Singleplayer row: your own server starts when nothing answers, a running one is reused, and one that can't start is
/// reported (the game then says "Failed to connect to server.").
#[tokio::test]
async fn singleplayer_starts_its_server_only_when_it_is_not_running() {
    let dir = scratch("server");
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    let cert = cfg.tls.certificate.clone();
    let missing = direct::ServerStart { program: dir.join("no-such-server").display().to_string(), ..Default::default() };
    let down = Endpoint::new("127.0.0.1:1".into(), "localhost".into(), &cert).unwrap();
    let error = direct::ensure_server(&down, &missing, Duration::from_secs(2)).await.err().unwrap();
    assert!(format!("{error:#}").contains("could not start"), "{error:#}");
    #[cfg(unix)] {
        let exits = direct::ServerStart { program: "/bin/true".into(), ..Default::default() };
        let error = direct::ensure_server(&down, &exits, Duration::from_secs(5)).await.err().unwrap();
        assert!(format!("{error:#}").contains("stopped during startup"), "{error:#}");
    }
    let server = yo3_server::start(cfg).await.unwrap();
    let up = Endpoint::new(server.auth.to_string(), "localhost".into(), &cert).unwrap();
    assert!(direct::ensure_server(&up, &missing, Duration::from_secs(2)).await.unwrap().is_none(), "a running server is reused");
    let command = direct::ServerStart { program: "yo3-server".into(), args: vec!["--data-dir".into(), "d".into()], ..Default::default() }.command();
    let args: Vec<_> = command.as_std().get_args().map(|a| a.to_string_lossy().into_owned()).collect();
    assert_eq!(args, ["--data-dir", "d", "--owned", "--owned-idle", "30"], "it stops itself 30 s after the game exits");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_direct_profile_names_its_mode() {
    let dir = scratch("profile"); std::fs::create_dir_all(&dir).unwrap();
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    let identity = yo3_tunnel::identity_from_pem(&std::fs::read(&cfg.tls.certificate).unwrap()).unwrap();
    let file = dir.join("direct.json");
    std::fs::write(&file, serde_json::json!({"server":"127.0.0.1:7443","server_name":"localhost","identity":identity,"vault":"server:abc"}).to_string()).unwrap();
    let loaded = yo3_launcher::Profile::read(&file).unwrap();
    assert!(matches!(&loaded.credential, Credential::Device(a) if a == "server:abc"));
    assert!(loaded.credential.owns_session(), "direct start ends the session it resumed");
    std::fs::write(&file, serde_json::json!({"server":"127.0.0.1:7443","server_name":"localhost","identity":identity,"key":"k",
        "start":{"program":"yo3-server","args":["--data-dir","d"],"env":{"YO3_ENGINE":"e"}}}).to_string()).unwrap();
    let loaded = yo3_launcher::Profile::read(&file).unwrap();
    assert!(matches!(&loaded.credential, Credential::Key(k) if k == "k"));
    assert_eq!(loaded.start.unwrap().env["YO3_ENGINE"], "e");
    let _ = std::fs::remove_dir_all(dir);
}
