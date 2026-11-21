//! GitHub HTTPS trust, independent of the operating system's root-certificate updates.
use reqwest::{Certificate, ClientBuilder};

fn with_roots(roots: impl IntoIterator<Item = Certificate>) -> ClientBuilder {
    let _ = yo3_tunnel::tokio_rustls::rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::builder().tls_certs_only(roots)
}

/// Only GitHub web requests use these roots. Game-server identity pinning stays in yo3-tunnel.
/// Refresh webpki-root-certs with releases; OS-installed private/interception CAs are not trusted here.
pub(crate) fn builder() -> ClientBuilder {
    with_roots(webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter().map(|der|
        Certificate::from_der(der.as_ref()).expect("bundled Mozilla certificate")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::Arc, time::Duration};
    use yo3_tunnel::tokio_rustls::{rustls, TlsAcceptor};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn request(name: &str, expired: bool, trust: bool) -> Result<reqwest::Response, reqwest::Error> {
        let key = rcgen::KeyPair::generate().unwrap();
        let mut ca = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
        ca.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let issuer = rcgen::Issuer::new(ca.clone(), &key);
        let root = ca.self_signed(&key).unwrap();
        let mut leaf = rcgen::CertificateParams::new(vec![name.into()]).unwrap();
        if expired { leaf.not_before = rcgen::date_time_ymd(2000, 1, 1); leaf.not_after = rcgen::date_time_ymd(2001, 1, 1); }
        let leaf_key = rcgen::KeyPair::generate().unwrap();
        let cert = leaf.signed_by(&leaf_key, &issuer).unwrap();
        let _ = rustls::crypto::ring::default_provider().install_default();
        let config = rustls::ServerConfig::builder().with_no_client_auth().with_single_cert(
            vec![cert.der().clone()], rustls::pki_types::PrivatePkcs8KeyDer::from(leaf_key.serialize_der()).into()).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            if let Ok(mut tls) = TlsAcceptor::from(Arc::new(config)).accept(tcp).await {
                let mut buf = [0; 4096];
                if tls.read(&mut buf).await.is_ok() { let _ = tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK").await; }
            }
        });
        let roots = if trust { vec![Certificate::from_der(root.der()).unwrap()] } else { vec![] };
        let result = with_roots(roots).https_only(true).no_proxy().timeout(Duration::from_secs(3))
            .resolve("localhost", address).build().unwrap().get(format!("https://localhost:{}/", address.port())).send().await;
        server.await.unwrap();
        result
    }

    #[tokio::test]
    async fn github_tls_accepts_bundled_trust_without_system_roots() {
        assert_eq!(request("localhost", false, true).await.unwrap().text().await.unwrap(), "OK");
    }
    #[tokio::test]
    async fn github_tls_rejects_untrusted_expired_and_wrong_host_certificates() {
        for (name, expired, trust, reason) in [("localhost", false, false, "UnknownIssuer"),
            ("localhost", true, true, "Expired"), ("wrong.example", false, true, "NotValidForName")] {
            let error = request(name, expired, trust).await.unwrap_err();
            assert!(format!("{error:?}").contains(reason), "{error:?}");
        }
    }
    /// Opt-in network check, also runnable as a native Windows test executable under VxKex.
    #[tokio::test]
    #[ignore = "requires access to GitHub"]
    async fn github_live_https_with_bundled_roots() {
        let client = builder().https_only(true).user_agent("yo3-revival-tls-check").timeout(Duration::from_secs(20)).build().unwrap();
        for url in ["https://api.github.com/repos/usersina/yo3-revival/releases/latest", "https://github.com/"] {
            let response = client.get(url).send().await.unwrap().error_for_status().unwrap();
            println!("{url}: {}", response.status());
        }
        println!("Update check: {:?}", crate::updates::check().await.unwrap());
        let root = std::env::temp_dir().join(format!("yo3-github-tls-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let servers = crate::multiplayer::official_list(&root).await;
        let _ = std::fs::remove_dir_all(&root);
        assert!(!servers.is_empty(), "official list did not download and verify");
        println!("Signed official list: {} server(s)", servers.len());
    }

}
