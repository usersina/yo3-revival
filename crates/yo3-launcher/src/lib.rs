//! Local, credential-free game endpoints. Every connection needs this launch's private cookie before it is relayed.
use std::net::Ipv4Addr;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use yo3_proto::{Message, Value, frame, frame_len, MAX_FRAME};
use yo3_tunnel::{Endpoint, Role, Token};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
pub mod components;
pub mod attach;

pub async fn authorize<R: AsyncRead + Unpin>(stream: &mut R, cookie: &Token) -> anyhow::Result<()> {
    let mut given = [0; 32];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut given)).await??;
    anyhow::ensure!(bool::from(given.ct_eq(cookie)), "local connection refused");
    Ok(())
}

#[derive(Deserialize)]
pub struct Profile { pub server: String, pub server_name: String, #[serde(default)] pub certificate: String,
    #[serde(default)] pub identity: Option<String>, pub key: String }

impl Profile {
    pub fn load(path: &Path) -> anyhow::Result<(Endpoint, String)> {
        let p: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        let certificate = path.parent().unwrap_or(Path::new(".")).join(p.certificate);
        let endpoint = match p.identity { Some(identity) => Endpoint::pinned(p.server, p.server_name, &identity)?,
            None => Endpoint::new(p.server, p.server_name, &certificate)? };
        Ok((endpoint, p.key))
    }
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct Ports { pub gate: u16, pub front: u16, pub duel: u16, pub http: u16 }

pub struct Proxy { pub ports: Ports, pub cookie: Token, tasks: Vec<JoinHandle<()>>, session: tokio::sync::watch::Sender<Option<Token>> }

impl Drop for Proxy { fn drop(&mut self) { for task in &self.tasks { task.abort(); } } }

impl Proxy {
    pub async fn start(endpoint: Endpoint, session: Token, ports: Ports) -> anyhow::Result<Self> {
        let proxy = Self::prepare(endpoint, ports).await?; proxy.activate(session); Ok(proxy)
    }
    pub fn activate(&self, session: Token) { self.session.send_replace(Some(session)); }
    pub async fn prepare(endpoint: Endpoint, ports: Ports) -> anyhow::Result<Self> {
        let (active, _) = tokio::sync::watch::channel(None);
        let gate = TcpListener::bind((Ipv4Addr::LOCALHOST, ports.gate)).await?;
        let front = TcpListener::bind((Ipv4Addr::LOCALHOST, ports.front)).await?;
        let duel = TcpListener::bind((Ipv4Addr::LOCALHOST, ports.duel)).await?;
        let http = TcpListener::bind((Ipv4Addr::LOCALHOST, ports.http)).await?;
        let ports = Ports { gate: gate.local_addr()?.port(), front: front.local_addr()?.port(), duel: duel.local_addr()?.port(), http: http.local_addr()?.port() };
        let mut cookie = [0; 32]; getrandom::fill(&mut cookie)?;
        let mut tasks = vec![];
        tasks.push(tokio::spawn(async move {
            while let Ok((mut s, _)) = gate.accept().await {
                tokio::spawn(async move {
                    let _ = async {
                        authorize(&mut s, &cookie).await?;
                        let mut head = [0; 4]; s.read_exact(&mut head).await?;
                        let n = frame_len(head); anyhow::ensure!(n > 0 && n <= MAX_FRAME, "bad gate frame");
                        let mut data = vec![0; n]; s.read_exact(&mut data).await?;
                        let reply = Message::reply(0x21FE).with("host", Value::text("127.0.0.1")).with("port", Value::U32(ports.front as u32));
                        s.write_all(&frame(&reply.encode())).await?;
                        Ok::<_, anyhow::Error>(())
                    }.await;
                });
            }
        }));
        for (listener, role) in [(front, Role::Front), (duel, Role::Duel)] {
            let endpoint = endpoint.clone();
            let active = active.clone();
            tasks.push(tokio::spawn(async move {
                while let Ok((mut local, _)) = listener.accept().await {
                    let endpoint = endpoint.clone();
                    let mut active = active.subscribe();
                    tokio::spawn(async move {
                        let result = async {
                            authorize(&mut local, &cookie).await?;
                            let session = wait_for_session(&mut active).await?;
                            let ticket = endpoint.ticket(&session, role).await?;
                            let mut remote = endpoint.connect(&ticket, role, ports.duel).await?;
                            tokio::io::copy_bidirectional(&mut local, &mut remote).await?;
                            Ok::<_, anyhow::Error>(())
                        }.await;
                        if let Err(e) = result { let message = format!("{role:?} connection: {e}"); yo3_tunnel::diagnostics::event(&message); eprintln!("{message}"); }
                    });
                }
            }));
        }
        let http_active = active.clone();
        tasks.push(tokio::spawn(async move {
            while let Ok((local, _)) = http.accept().await {
                let endpoint = endpoint.clone();
                let mut active = http_active.subscribe();
                tokio::spawn(async move { if let Ok(session) = wait_for_session(&mut active).await { let _ = relay_http(local, &endpoint, &session, &cookie).await; } });
            }
        }));
        Ok(Self { ports, cookie, tasks, session: active })
    }

    pub fn environment(&self) -> Vec<(&'static str, String)> {
        [("YO3_LOCAL_GATE", self.ports.gate), ("YO3_LOCAL_FRONT", self.ports.front), ("YO3_LOCAL_DUEL", self.ports.duel), ("YO3_LOCAL_HTTP", self.ports.http)]
            .into_iter().map(|(k, v)| (k, v.to_string())).chain(std::iter::once(("YO3_LOCAL_COOKIE", hex(&self.cookie)))).collect()
    }
}

async fn wait_for_session(active: &mut tokio::sync::watch::Receiver<Option<Token>>) -> anyhow::Result<Token> {
    let token = *tokio::time::timeout(Duration::from_secs(10), active.wait_for(|session| session.is_some())).await??;
    token.ok_or_else(|| anyhow::anyhow!("launcher session is not ready"))
}

pub fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }

/// The desktop owns this pipe: a stop command or its exit must release the authenticated game session.
pub async fn wait_for_desktop_stop(mut input: impl std::io::BufRead + Send + 'static) {
    let (done, stopped) = tokio::sync::oneshot::channel();
    // Tokio's stdin uses a blocking runtime task that cannot be cancelled. A detached OS thread instead lets
    // the launcher finish naturally even while the desktop still holds its input pipe open.
    std::thread::spawn(move || {
        let mut line = String::new();
        loop {
            line.clear();
            match input.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) if line.trim() == "stop" => break,
                Ok(_) => {},
            }
        }
        let _ = done.send(());
    });
    let _ = stopped.await;
}

/// The browser/WinINet uses the launch cookie in a local path, never an account credential in a remote URL.
async fn relay_http(mut local: TcpStream, endpoint: &Endpoint, session: &Token, cookie: &Token) -> anyhow::Result<()> {
    let mut request = vec![];
    tokio::time::timeout(Duration::from_secs(5), async {
        while request.len() < 8192 && !request.ends_with(b"\r\n\r\n") {
            request.push(local.read_u8().await?);
        }
        Ok::<_, std::io::Error>(())
    }).await??;
    let text = std::str::from_utf8(&request)?;
    let path = text.split_whitespace().nth(1).unwrap_or_default();
    let prefix = format!("/session/{}/", hex(cookie));
    let path = path.strip_prefix(&prefix).filter(|_| text.starts_with("GET ")).ok_or_else(|| anyhow::anyhow!("local HTTP refused"))?;
    anyhow::ensure!(!path.starts_with("admin") && !path.starts_with("join"), "private HTTP route refused");
    let ticket = endpoint.ticket(session, Role::Http).await?;
    let mut remote = endpoint.connect(&ticket, Role::Http, 0).await?;
    remote.write_all(format!("GET /{path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").as_bytes()).await?;
    tokio::io::copy(&mut remote, &mut local).await?;
    local.shutdown().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn desktop_control_stops_on_command_or_parent_exit() {
        let task = tokio::spawn(wait_for_desktop_stop(std::io::Cursor::new(Vec::<u8>::new())));
        tokio::time::timeout(Duration::from_secs(1), task).await.unwrap().unwrap();
        let task = tokio::spawn(wait_for_desktop_stop(std::io::Cursor::new(b"ignored\nstop\n")));
        tokio::time::timeout(Duration::from_secs(1), task).await.unwrap().unwrap();
    }
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn a_local_socket_without_this_launchs_cookie_is_refused() {
        let (mut a, mut b) = tokio::io::duplex(128);
        a.write_all(&[7; 32]).await.unwrap();
        assert!(authorize(&mut b, &[8; 32]).await.is_err());
    }

    #[tokio::test]
    async fn authorizing_a_local_socket_preserves_the_games_first_bytes() {
        let (mut a, mut b) = tokio::io::duplex(128);
        a.write_all(&[8; 32]).await.unwrap();
        a.write_all(b"game").await.unwrap();
        authorize(&mut b, &[8; 32]).await.unwrap();
        let mut data = [0; 4]; b.read_exact(&mut data).await.unwrap();
        assert_eq!(&data, b"game");
    }
}
