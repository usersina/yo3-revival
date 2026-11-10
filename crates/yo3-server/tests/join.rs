//! Plain HTTP must no longer create accounts, reveal account keys, or serve private client profiles.
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use yo3_server::{Config, start};

#[tokio::test]
async fn the_retired_ip_based_join_page_cannot_disclose_or_create_credentials() {
    let dir = std::env::temp_dir().join(format!("yo3-no-join-{}", std::process::id()));
    let cfg = Config::ephemeral(dir.clone());
    let file = cfg.data_dir.clone().unwrap().join("accounts.json");
    let before = std::fs::read(&file).unwrap();
    let server = start(cfg).await.unwrap();
    for path in ["/join", "/join?name=Eve", "/join/yo3.ini", "/join?keep=1"] {
        let mut s = TcpStream::connect(server.http).await.unwrap();
        s.write_all(format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes()).await.unwrap();
        let mut reply = String::new(); s.read_to_string(&mut reply).await.unwrap();
        assert!(reply.starts_with("HTTP/1.1 404"), "{path}: {reply}");
    }
    assert_eq!(std::fs::read(file).unwrap(), before);
}
