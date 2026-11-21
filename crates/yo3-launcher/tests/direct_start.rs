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
    let made = endpoint.account(&serde_json::json!({ "op": "register", "user": "direct", "password": "LongPassphrase1", "rules": true })).await.unwrap();
    assert_eq!(made["ok"], true, "{made}");
    let signed = endpoint.account(&serde_json::json!({ "op": "login", "user": "direct", "password": "LongPassphrase1", "stay": true, "device": "test" })).await.unwrap();
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

async fn http(proxy: &Proxy, request: String) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", proxy.ports.http)).await.unwrap();
    s.write_all(request.as_bytes()).await.unwrap();
    let mut reply = vec![]; let _ = s.read_to_end(&mut reply).await;
    String::from_utf8(reply).unwrap()
}

/// The real game asks for its patch index (yo3.idx, through WinINet) as it starts, before its title screen. Without a
/// sign-in there is no session to fetch it with: found on Windows 11, the game then said "You will need an active
/// Internet connection to play ONLINE." and closed, so the login form (#34) or the refusal never showed. A refused
/// start answers with the game's own index (nothing to update); a signed-in one still asks the server.
#[tokio::test]
async fn a_refused_start_answers_the_patch_check_with_the_games_own_index() {
    let dir = scratch("index");
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    let cert = cfg.tls.certificate.clone();
    let server = yo3_server::start(cfg).await.unwrap();
    let endpoint = Endpoint::new(server.auth.to_string(), "localhost".into(), &cert).unwrap();
    let proxy = Proxy::prepare(endpoint, Ports::default()).await.unwrap();
    let cookie = yo3_launcher::hex(&proxy.cookie);
    let index = format!("GET /session/{cookie}/yo3/yo3.idx HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let other = format!("GET /session/{cookie}/web/news HTTP/1.1\r\nHost: localhost\r\n\r\n");
    proxy.local_index(b"the game's own index".to_vec());
    for refusal in [Refusal::SignedOut, Refusal::FailedToConnect] {
        proxy.refuse(refusal);
        let reply = http(&proxy, index.clone()).await;
        assert!(reply.starts_with("HTTP/1.1 200") && reply.ends_with("\r\n\r\nthe game's own index"), "{refusal:?}: {reply}");
        // Nothing of the server's is fetched without a sign-in: a browser's page says so (#74), the game's requests end.
        let page = http(&proxy, other.clone()).await;
        assert!(page.starts_with("HTTP/1.1 403") && page.contains("not signed in") && !page.contains("Konami"), "{refusal:?}: {page}");
        let log = format!("GET /session/{cookie}/duellogs/x.dlog HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert!(http(&proxy, log).await.is_empty(), "{refusal:?}: nothing else is answered without a sign-in");
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// #34: the game's own login form shows only when there is no saved sign-in, and what the player types there reaches
/// the helper, which signs in with it (Save Password: the device keeps a token) or lets the form show the refusal.
#[tokio::test]
async fn the_games_login_form_signs_in_with_what_the_player_typed() {
    yo3_server::logins::use_fast_hashing();
    let dir = scratch("typed");
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    let cert = cfg.tls.certificate.clone();
    let server = yo3_server::start(cfg).await.unwrap();
    let endpoint = Endpoint::new(server.auth.to_string(), "localhost".into(), &cert).unwrap();
    let made = endpoint.account(&serde_json::json!({ "op": "register", "user": "former", "password": "LongPassphrase1", "rules": true })).await.unwrap();
    assert_eq!(made["ok"], true, "{made}");
    let proxy = Proxy::prepare(endpoint.clone(), Ports::default()).await.unwrap();
    let cookie = yo3_launcher::hex(&proxy.cookie);
    let state = format!("GET /session/{cookie}/{} HTTP/1.1\r\nHost: localhost\r\n\r\n", yo3_launcher::LOGIN_STATE_PATH);
    proxy.refuse(Refusal::FailedToConnect);
    assert!(http(&proxy, state.clone()).await.ends_with("\r\n\r\nready"), "a server that is down: no form, the game says it can't connect");
    proxy.refuse(Refusal::SignedOut);
    assert!(http(&proxy, state.clone()).await.ends_with("\r\n\r\nform"), "no saved sign-in: the game's own form");
    proxy.refuse_typed(Refusal::WrongPassword);
    assert!(http(&proxy, state.clone()).await.ends_with("\r\n\r\nform"), "after a wrong password the next ONLINE shows the form again");
    proxy.activate_typed([3; 32]);
    assert!(http(&proxy, state.clone()).await.ends_with("\r\n\r\nready"), "signed in: ONLINE goes straight on");
    proxy.refuse(Refusal::SignedOut);
    // What the DLL posts when the game connects to the front after goLogin.
    let mut typed = proxy.typed_logins().unwrap();
    let body = serde_json::json!({ "user": "former", "password": "LongPassphrase1", "stay": true }).to_string();
    let reply = http(&proxy, format!("POST /session/{cookie}/{} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{body}",
        yo3_launcher::LOGIN_PATH, body.len())).await;
    assert!(reply.starts_with("HTTP/1.1 204"), "{reply}");
    let login = tokio::time::timeout(Duration::from_secs(1), typed.recv()).await.unwrap().unwrap();
    assert!(!format!("{login:?}").contains("passphrase"), "a password never reaches a log");
    let wrong = http(&proxy, format!("POST /session/00/{} HTTP/1.1\r\nContent-Length: 2\r\n\r\n{{}}", yo3_launcher::LOGIN_PATH)).await;
    assert!(wrong.is_empty(), "another process without the cookie is refused");
    let vault = MemoryVault::default();
    direct::sign_in_typed(&endpoint, &login, Some("server:typed"), &vault).await.unwrap();
    let token = vault.get("server:typed").unwrap().expect("Save Password keeps a device token");
    let read = |a: &str| vault.get(a).unwrap();
    direct::sign_in(&endpoint, &Credential::Device("server:typed".into()), &read).await.unwrap();
    assert_eq!(token.len(), 64);
    let plain = direct::Typed { user: "former".into(), password: "LongPassphrase1".into(), stay: false };
    let other = MemoryVault::default();
    direct::sign_in_typed(&endpoint, &plain, Some("server:typed"), &other).await.unwrap();
    assert_eq!(other.get("server:typed").unwrap(), None, "without Save Password nothing is kept");
    let bad = direct::Typed { user: "former".into(), password: "not the passphrase".into(), stay: true };
    assert_eq!(direct::sign_in_typed(&endpoint, &bad, Some("server:typed"), &other).await.unwrap_err().refusal, Refusal::WrongPassword);
    let down = Endpoint::new("127.0.0.1:1".into(), "localhost".into(), &cert).unwrap();
    assert_eq!(direct::sign_in_typed(&down, &plain, None, &other).await.unwrap_err().refusal, Refusal::FailedToConnect);
    let _ = std::fs::remove_dir_all(dir);
}

/// Singleplayer's key lives in the system's credential store (#23): the profile names the entry (`key_vault`), and
/// the helper reads the key from there. A profile still holding its key (no store on that PC) plays as before.
#[test]
fn a_singleplayer_profile_finds_its_key_in_the_credential_store() {
    let dir = scratch("key-vault"); std::fs::create_dir_all(&dir).unwrap();
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    let identity = yo3_tunnel::identity_from_pem(&std::fs::read(&cfg.tls.certificate).unwrap()).unwrap();
    let entry = yo3_launcher::vault::key_account(&identity);
    assert_eq!(entry, format!("key:{identity}"));
    let file = dir.join("connection-0.json");
    std::fs::write(&file, serde_json::json!({"server":"127.0.0.1:7443","server_name":"localhost","identity":identity,"key_vault":entry}).to_string()).unwrap();
    let vault = MemoryVault::default();
    let error = yo3_launcher::Profile::read_with(&file, &vault).err().expect("no key in the store");
    assert!(format!("{error:#}").contains("credential store"), "{error:#}");
    vault.set(&entry, "abcdEFGH2345ijkmNPQR").unwrap();
    let loaded = yo3_launcher::Profile::read_with(&file, &vault).unwrap();
    assert!(matches!(&loaded.credential, Credential::Key(k) if k == "abcdEFGH2345ijkmNPQR"));
    assert_eq!(loaded.vault, None, "not a saved sign-in: Delete Login Data has nothing to forget");
    std::fs::write(&file, serde_json::json!({"server":"127.0.0.1:7443","server_name":"localhost","identity":identity,"key":"k"}).to_string()).unwrap();
    assert!(matches!(yo3_launcher::Profile::read_with(&file, &MemoryVault::default()).unwrap().credential, Credential::Key(k) if k == "k"));
    let _ = std::fs::remove_dir_all(dir);
}
