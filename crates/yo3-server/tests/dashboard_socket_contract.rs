//! A mounted Unix socket grants only the staff router, never local host authority.
#![cfg(unix)]
use tokio::io::{AsyncReadExt,AsyncWriteExt};
use yo3_server::{Config,start};

async fn request(path:&std::path::Path, host:&str, extra:&str, route:&str)->String{
    let mut s=tokio::net::UnixStream::connect(path).await.unwrap();
    s.write_all(format!("GET {route} HTTP/1.1\r\nHost: {host}\r\n{extra}Connection: close\r\n\r\n").as_bytes()).await.unwrap();
    let mut out=String::new();s.read_to_string(&mut out).await.unwrap();out
}
#[tokio::test]
async fn unix_proxy_validates_origin_and_forwarding_without_host_authority(){
    yo3_server::logins::use_fast_hashing();
    let root=std::env::temp_dir().join(format!("yo3-unix-proxy-{}",std::process::id()));
    let _=std::fs::remove_dir_all(&root);std::fs::create_dir_all(&root).unwrap();
    let socket=root.join("staff.sock");let mut cfg=Config::ephemeral(root.join("logs"));
    cfg.dashboard_socket=Some(socket.clone());cfg.dashboard_origin=Some("https://staff.example".into());
    let cert=cfg.tls.certificate.clone();
    let live=start(cfg).await.unwrap();assert!(live.dashboard.is_none());
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(std::fs::metadata(&socket).unwrap().permissions().mode()&0o777,0o660);
    let headers="X-Forwarded-For: 198.51.100.7\r\nX-Forwarded-Proto: https\r\n";
    let ok=request(&socket,"staff.example",headers,"/panel/openapi.json").await;assert!(ok.starts_with("HTTP/1.1 200"),"{ok}");
    for (host,forward) in [("evil.example",headers),("staff.example",""),("staff.example","X-Forwarded-For: 127.0.0.1, 198.51.100.7\r\nX-Forwarded-Proto: https\r\n"),("staff.example","X-Forwarded-For: 198.51.100.7\r\nX-Forwarded-Proto: http\r\n"),("staff.example","X-Forwarded-For: 198.51.100.7\r\nX-Forwarded-Proto: https\r\nForwarded: for=127.0.0.1\r\n")]{
        let bad=request(&socket,host,forward,"/panel/openapi.json").await;assert!(bad.starts_with("HTTP/1.1 403"),"{bad}");
    }
    for route in ["/api/admin/v1/status","/panel/api/v1/restore","/panel/","/health"]{
        let bad=request(&socket,"staff.example",headers,route).await;assert!(bad.starts_with("HTTP/1.1 404"),"{bad}");
    }
    let unauth=request(&socket,"staff.example",headers,"/panel/api/v1/accounts").await;assert!(unauth.starts_with("HTTP/1.1 401"));
    let endpoint=yo3_tunnel::Endpoint::new(live.auth.to_string(),"localhost".into(),&cert).unwrap();
    assert_eq!(endpoint.account(&serde_json::json!({"op":"register","user":"SocketAdmin","password":"SocketPassword1","rules":true})).await.unwrap()["ok"],true);
    let admin=yo3_server::admin::running(&root.join("logs/test-auth/data")).unwrap();
    tokio::task::spawn_blocking(move||yo3_server::api::call(&admin,&yo3_server::api::Op::SetRole{user:"SocketAdmin".into(),role:"admin".into()})).await.unwrap().unwrap();
    for (origin,expected) in [("https://evil.example",403),("https://staff.example",200)]{
        let mut stream=tokio::net::UnixStream::connect(&socket).await.unwrap();
        let body=r#"{"user":"SocketAdmin","password":"SocketPassword1"}"#;
        stream.write_all(format!("POST /panel/api/v1/sign-in HTTP/1.1\r\nHost: staff.example\r\n{headers}Origin: {origin}\r\nIdempotency-Key: socket-signin-test-0001\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        let mut out=String::new();stream.read_to_string(&mut out).await.unwrap();assert!(out.starts_with(&format!("HTTP/1.1 {expected}")),"{out}");
        if expected==200{assert!(out.contains("HttpOnly; Secure; SameSite=Strict"));}
    }
    live.stopper().send_replace(true);live.wait().await;
    assert!(!socket.exists(),"graceful stop removes only its own socket");
    yo3_server::store::close(&root.join("logs/test-auth/data"));let _=std::fs::remove_dir_all(root);
}
#[tokio::test]
async fn invalid_proxy_configuration_and_existing_files_fail_before_storage(){
    for (name,origin) in [("plain","http://staff.example"),("path","https://staff.example/path"),("userinfo","https://user@staff.example"),("query","https://staff.example?x")]{
        let root=std::env::temp_dir().join(format!("yo3-unix-invalid-{name}-{}",std::process::id()));
        let mut cfg=Config::ephemeral(root.clone());cfg.data_dir=Some(root.join("unopened"));
        cfg.dashboard_socket=Some(root.join("staff.sock"));cfg.dashboard_origin=Some(origin.into());
        assert!(start(cfg).await.is_err());assert!(!root.join("unopened/server.db").exists());let _=std::fs::remove_dir_all(root);
    }
    let root=std::env::temp_dir().join(format!("yo3-unix-file-{}",std::process::id()));let mut cfg=Config::ephemeral(root.clone());
    let path=root.join("staff.sock");std::fs::write(&path,"preserve").unwrap();cfg.dashboard_socket=Some(path.clone());cfg.dashboard_origin=Some("https://staff.example".into());
    assert!(start(cfg).await.is_err());assert_eq!(std::fs::read_to_string(path).unwrap(),"preserve");let _=std::fs::remove_dir_all(root);
}
#[tokio::test]
async fn crashed_socket_is_reclaimed_but_a_live_socket_is_never_unlinked(){
    let root=std::env::temp_dir().join(format!("yo3-unix-stale-{}",std::process::id()));let cfg=Config::ephemeral(root.clone());let path=root.join("staff.sock");
    {let stale=std::os::unix::net::UnixListener::bind(&path).unwrap();drop(stale);}
    let mut cfg=cfg;cfg.dashboard_socket=Some(path.clone());cfg.dashboard_origin=Some("https://staff.example".into());
    let live=start(cfg.clone()).await.unwrap();
    cfg.tls.port=0;cfg.http_port=0;assert!(start(cfg).await.is_err());assert!(path.exists());
    live.stopper().send_replace(true);live.wait().await;let _=std::fs::remove_dir_all(root);
}
