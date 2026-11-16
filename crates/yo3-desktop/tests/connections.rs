//! Singleplayer's key connection: your own server's identity and your local account's key, including a certificate
//! without localhost in its names.
use std::path::Path;
use yo3_desktop::{Backend, Settings};
use yo3_server::{Config, admin, api::{self, Op}};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_local_connection_keeps_the_certificate_identity_when_dialling_the_server_locally() {
    let root = std::env::temp_dir().join(format!("yo3-desktop-connections-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut cfg = Config::ephemeral(root.join("host"));
    let data = cfg.data_dir.clone().unwrap();
    let identity = yo3_server::auth::initialize(&root.join("named-identity"), vec!["friends.example".into()]).unwrap();
    cfg.tls.certificate = identity.certificate.clone(); cfg.tls.private_key = identity.private_key;
    std::fs::copy(&identity.certificate, data.join("server.crt")).unwrap();
    let server = yo3_server::start(cfg).await.unwrap();
    let settings = Settings { host_address: format!("friends.example:{}", server.auth.port()), data_dir: data.to_string_lossy().into(), ..Settings::default() };
    let backend = Backend::new(root.join("desktop")).unwrap();
    let worker = backend.clone();
    tokio::task::spawn_blocking(move || {
        worker.manage(&settings, Op::CreateAccount { names: vec!["Guest".into()] }).unwrap();
        let local_only = Settings { host_address: "localhost".into(), ..settings.clone() };
        worker.local_connection(&local_only, "Guest").unwrap();
        let mut local = worker.local_connection(&settings, "Guest").unwrap();
        assert_eq!(local.server_name, "friends.example", "loopback routing must not replace the certificate identity");
        local.validate().unwrap();
        let sign_in = |connection: &yo3_desktop::KeyConnection| {
            let endpoint = connection.endpoint().unwrap();
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(endpoint.login(&connection.key))
        };
        sign_in(&local).unwrap();
        let key = local.key.clone();
        local.key = "x".repeat(20); assert!(sign_in(&local).is_err(), "a wrong key is refused");
        local.key = key; local.identity = Some("00".repeat(32)); assert!(sign_in(&local).is_err(), "another identity is refused");
        assert!(Path::new(&settings.data_dir).join("server.db").exists());
        let live = admin::running(Path::new(&settings.data_dir)).unwrap(); api::call(&live, &Op::Shutdown).unwrap();
    }).await.unwrap();
    server.wait().await;
}
