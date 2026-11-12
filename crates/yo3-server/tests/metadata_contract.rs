//! Discovery precedes credentials; compatible patch releases need not have identical build labels.
use yo3_tunnel::{Endpoint, MOD_ABI, PROTOCOL_VERSION};

#[tokio::test]
async fn discovery_reports_requirements_without_an_account_and_rejects_incompatible_versions() {
    let root = std::env::temp_dir().join(format!("yo3-discovery-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cfg = yo3_server::Config::ephemeral(root);
    let certificate = cfg.tls.certificate.clone();
    let server = yo3_server::start(cfg).await.unwrap();
    let endpoint = Endpoint::new(server.auth.to_string(), "localhost".into(), &certificate).unwrap();
    let info = endpoint.info().await.unwrap();
    assert!(!info.release.is_empty());
    assert_eq!(info.protocol, PROTOCOL_VERSION);
    assert_eq!(info.mod_abi, MOD_ABI);
    assert_eq!(info.game_version, "1.153");
    info.ensure_compatible(PROTOCOL_VERSION, MOD_ABI).unwrap();
    assert!(info.ensure_compatible(PROTOCOL_VERSION + 1, MOD_ABI).is_err());
    assert!(info.ensure_compatible(PROTOCOL_VERSION, MOD_ABI + 1).is_err());
    let mut patch = info.clone(); patch.release = "another-compatible-patch".into();
    patch.ensure_compatible(PROTOCOL_VERSION, MOD_ABI).unwrap();
    let wrong = Endpoint::new(server.auth.to_string(), "wrong.example".into(), &certificate).unwrap();
    assert!(wrong.info().await.is_err(), "discovery must verify the server identity too");
}

#[tokio::test]
async fn address_discovery_does_not_authenticate_and_pins_the_hosts_key_across_certificate_renewal() {
    let root = std::env::temp_dir().join(format!("yo3-identity-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut cfg = yo3_server::Config::ephemeral(root.clone());
    let identity = yo3_server::auth::initialize(&root.join("identity"), vec!["friends.example".into()]).unwrap();
    cfg.tls = yo3_server::auth::TlsConfig { port: 0, ..identity.clone() };
    let server = yo3_server::start(cfg).await.unwrap();
    let observed = yo3_tunnel::discover(&server.auth.to_string(), "127.0.0.1").await.unwrap();
    assert!(!observed.identity.is_empty());
    assert!(observed.certificate.contains("BEGIN CERTIFICATE"));
    // Address discovery sends only metadata, with no account key or automatic trust decision.
    let pinned = Endpoint::pinned(server.auth.to_string(), "127.0.0.1".into(), &observed.identity).unwrap();
    pinned.info().await.unwrap();
    let wrong = Endpoint::pinned(server.auth.to_string(), "127.0.0.1".into(), &"00".repeat(32)).unwrap();
    assert!(wrong.info().await.is_err());
    yo3_server::auth::reissue_certificate(&root.join("identity"), vec!["new-address.example".into()]).unwrap();
    assert_eq!(yo3_tunnel::identity_from_pem(&std::fs::read(&identity.certificate).unwrap()).unwrap(), observed.identity);
    let mut renewed = yo3_server::Config::ephemeral(root.join("renewed"));
    renewed.tls = yo3_server::auth::TlsConfig { port: 0, ..identity };
    let renewed = yo3_server::start(renewed).await.unwrap();
    Endpoint::pinned(renewed.auth.to_string(), "127.0.0.1".into(), &observed.identity).unwrap().info().await.unwrap();
}

#[tokio::test]
async fn a_command_line_profile_keeps_working_after_the_server_renews_its_certificate() {
    // Scripts profiles carry a copy of server.crt as their trust anchor. A start-up renewal (auth::renew_on_start)
    // keeps the key, names and subject, so the friend's old copy still verifies the renewed certificate.
    let root = std::env::temp_dir().join(format!("yo3-cli-renewal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let identity = yo3_server::auth::initialize(&root.join("identity"), vec!["localhost".into()]).unwrap();
    let friends_copy = root.join("friend-server.crt");
    std::fs::copy(&identity.certificate, &friends_copy).unwrap();
    yo3_server::auth::reissue_certificate(&root.join("identity"), vec!["localhost".into()]).unwrap();
    assert_ne!(std::fs::read(&identity.certificate).unwrap(), std::fs::read(&friends_copy).unwrap());
    let mut cfg = yo3_server::Config::ephemeral(root.join("server"));
    cfg.tls = yo3_server::auth::TlsConfig { port: 0, ..identity };
    let server = yo3_server::start(cfg).await.unwrap();
    Endpoint::new(server.auth.to_string(), "localhost".into(), &friends_copy).unwrap().info().await.unwrap();
}

#[tokio::test]
async fn expired_identities_and_oversized_metadata_are_refused() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let root = std::env::temp_dir().join(format!("yo3-expired-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root); std::fs::create_dir_all(&root).unwrap();
    let key = rcgen::KeyPair::generate().unwrap();
    let mut params = rcgen::CertificateParams::new(vec!["localhost".into()]).unwrap();
    params.not_before = rcgen::date_time_ymd(2000, 1, 1); params.not_after = rcgen::date_time_ymd(2001, 1, 1);
    let cert = root.join("expired.crt"); let private = root.join("expired.key");
    std::fs::write(&cert, params.self_signed(&key).unwrap().pem()).unwrap(); std::fs::write(&private, key.serialize_pem()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(); let address = listener.local_addr().unwrap();
    let acceptor = yo3_tunnel::acceptor(&cert, &private).unwrap();
    let task = tokio::spawn(async move { let (tcp, _) = listener.accept().await.unwrap(); let _ = acceptor.accept(tcp).await; });
    assert!(yo3_tunnel::discover(&address.to_string(), "localhost").await.is_err()); task.await.unwrap();
    let tls = yo3_server::auth::initialize(&root.join("valid"), vec!["localhost".into()]).unwrap();
    let endpoint_cert = tls.certificate.clone(); let acceptor = yo3_tunnel::acceptor(&tls.certificate, &tls.private_key).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(); let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap(); let mut tls = acceptor.accept(tcp).await.unwrap();
        let mut header = [0; 9]; tls.read_exact(&mut header).await.unwrap(); assert_eq!(header[8], 5);
        tls.write_u16((yo3_tunnel::MAX_INFO + 1) as u16).await.unwrap();
    });
    assert!(Endpoint::new(address.to_string(), "localhost".into(), &endpoint_cert).unwrap().info().await.is_err()); task.await.unwrap();
}
