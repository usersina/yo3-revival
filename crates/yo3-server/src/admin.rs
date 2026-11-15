//! How the host's tools find a running server: at start it writes [`LIVE_FILE`] into its data directory, with its
//! loopback HTTP port and a random token, readable by the host's user only. The CLI and the desktop app read it and
//! send their operations to the admin API (`api.rs`) there, with the token. With no server answering, the CLI applies
//! the operation to the data folder itself.
use std::io::{self, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use serde_json::json;

/// An HTTP answer of the loopback service.
pub struct Response {
    pub status: &'static str,
    pub content_type: &'static str,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

/// The running server's port and token, in its data directory.
pub const LIVE_FILE: &str = "admin.json";

/// Where a running server takes admin operations.
#[derive(Clone, Debug, PartialEq)]
pub struct Live {
    pub port: u16,
    pub token: String,
}

/// Writes [`LIVE_FILE`] for a server on `port` (readable by the host's user only); its token.
pub fn write_live(data_dir: &Path, port: u16) -> io::Result<String> {
    let token = crate::accounts::new_key();
    std::fs::create_dir_all(data_dir)?;
    let path = data_dir.join(LIVE_FILE);
    let _ = std::fs::remove_file(&path);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(&path)?.write_all(json!({ "port": port, "token": token }).to_string().as_bytes())?;
    Ok(token)
}

/// The running server of `data_dir`, if its [`LIVE_FILE`] is there and the server answers on its port.
pub fn running(data_dir: &Path) -> Option<Live> {
    let text = std::fs::read_to_string(data_dir.join(LIVE_FILE)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let live = Live { port: v["port"].as_u64()? as u16, token: v["token"].as_str()?.to_string() };
    TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], live.port)), Duration::from_secs(2)).ok()?;
    Some(live)
}

/// Compares two tokens in time that doesn't depend on where they differ.
pub(crate) fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
