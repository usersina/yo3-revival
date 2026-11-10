//! Desktop invitations carry the host identity and account together, including certificates without localhost.
use std::path::Path;
use yo3_desktop::{Backend, LauncherDb, SavedServer, Settings};
use yo3_server::{Config, admin::{self, ManagerOp}};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invitation_roundtrip_keeps_remote_identity_when_dialling_the_host_locally() {
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
        worker.manage(&settings, ManagerOp::CreateAccount { name: "Guest".into() }).unwrap();
        let local = worker.local_connection(&settings, "Guest").unwrap();
        assert_eq!(local.server_name, "friends.example", "loopback routing must not replace the certificate identity");
        worker.check_connection(&local).unwrap();
        let file = worker.root.join("invite.json"); local.export(&file).unwrap();
        assert!(local.export(&file).is_err(), "existing invitation is never overwritten silently");
        let imported = SavedServer::import(&file).unwrap();
        assert_eq!(imported.key, local.key);
        let db = LauncherDb::open(&worker.root).unwrap(); db.save_server(&imported).unwrap();
        let mut remembered = db.servers().unwrap().remove(0);
        worker.check_connection(&remembered).unwrap();
        remembered.key = "x".repeat(20); assert!(worker.check_connection(&remembered).is_err());
        remembered.key = local.key; remembered.identity = Some("00".repeat(32)); assert!(worker.check_connection(&remembered).is_err());
        assert!(worker.root.join("launcher.db").exists());
        assert!(Path::new(&settings.data_dir).join("server.db").exists());
        let live = admin::running(Path::new(&settings.data_dir)).unwrap(); admin::manage(&live, &ManagerOp::Shutdown).unwrap();
    }).await.unwrap();
    server.wait().await;
}
