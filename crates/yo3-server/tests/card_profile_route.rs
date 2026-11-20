//! The card profile over the launcher's authenticated HTTP role (#60): `GET /web/card-profile` answers the connection
//! DLL with the selected profile's client file (empty for the game's own cards), whatever the account. Skipped unless
//! the game is installed and engine/fetch.sh has run.
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yo3_server::auth::TlsConfig;
use yo3_server::profile::Profile;
use yo3_server::{Config, start};
use yo3_tunnel::{Endpoint, Role};

async fn fetch(anime: &str) -> Option<(String, String)> {
    let engine = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine");
    if !yo3_data::default_game_dir().join("package/data002.ypk").is_file() || yo3_server::engine::anime_problem(&engine).is_some() {
        eprintln!("game or anime data missing; skipping");
        return None;
    }
    // SAFETY: set before the server starts and read by it alone; this test binary has one test.
    unsafe { std::env::set_var("YO3_ENGINE_DIR", &engine) };
    let dir = std::env::temp_dir().join(format!("yo3-profile-{anime}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let (certificate, private_key) = (dir.join("server.crt"), dir.join("server.key"));
    std::fs::write(&certificate, cert.cert.pem()).unwrap();
    std::fs::write(&private_key, cert.signing_key.serialize_pem()).unwrap();
    yo3_server::store::initialize(&dir).unwrap();
    let mut d = yo3_server::accounts::Directory::open(vec![], &dir.join("accounts.json")).unwrap();
    let key = d.add_account(&["Alice".into()]).unwrap();
    drop(d);
    let mut cfg = Config { tls: TlsConfig { port: 0, certificate: certificate.clone(), private_key },
        data_dir: Some(dir.clone()), game_dir: yo3_data::default_game_dir(), allow_missing_game_data: false, ..Config::ephemeral(dir.join("logs")) };
    cfg.settings.push(("anime-cards".into(), anime.into()));
    let server = start(cfg).await.unwrap();
    let endpoint = Endpoint::new(server.auth.to_string(), "localhost".into(), &certificate).unwrap();
    // Discovery names the profile too, for the launcher's restart offer.
    let announced = endpoint.info().await.unwrap().card_profile;
    let session = endpoint.login(&key).await.unwrap();
    let ticket = endpoint.ticket(&session, Role::Http).await.unwrap();
    let mut tunnel = endpoint.connect(&ticket, Role::Http, 0).await.unwrap();
    tunnel.write_all(b"GET /web/card-profile HTTP/1.1\r\nHost: x\r\n\r\n").await.unwrap();
    let mut out = Vec::new();
    let _ = tunnel.read_to_end(&mut out).await;
    let _ = std::fs::remove_dir_all(&dir);
    Some((String::from_utf8_lossy(&out).into_owned(), announced))
}

#[tokio::test]
async fn the_launcher_gets_the_selected_profile_and_the_game_s_own_by_default() {
    let Some((anime, announced)) = fetch("on").await else { return };
    let (head, body) = anime.split_once("\r\n\r\n").expect("a response");
    assert!(head.starts_with("HTTP/1.1 200") && head.contains("Content-Length"), "{head}");
    let (name, hash) = Profile::identify(body).expect("a client file");
    assert_eq!(name, "anime");
    assert_eq!(hash.len(), 16);
    assert_eq!(announced, hash, "discovery announces the hash of the file the game fetches");
    assert!(body.contains("\ncard ") && body.contains("\ntext ") && body.contains("\nart "));
    let (own, announced) = fetch("off").await.unwrap();
    assert_eq!(announced, "", "the game's own cards announce no profile");
    let (head, body) = own.split_once("\r\n\r\n").expect("a response");
    assert!(head.starts_with("HTTP/1.1 200") && head.contains("Content-Length: 0"), "{head}");
    assert!(body.is_empty(), "the game's own cards: an empty file");
}
