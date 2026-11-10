//! Connection loops: framing, decryption, logging, and the patch-HTTP endpoint.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use yo3_proto::{MAX_FRAME, Message, Packet, decrypt, frame, frame_len};

use crate::accounts::Directory;
use crate::front::Outbox;
use crate::log::PacketLog;

/// How long a connection may stay silent before it is closed: before its first message on every port (a socket that
/// never speaks), inside a frame, and between messages on the duel connection, where the real client heartbeats every
/// 30 s (`0x2724`). On the gate and front connections a client that has spoken may stay quiet: it sends nothing while
/// it idles on the map (gaps of 650 s and 3000 s in the Windows VM test's packet logs).
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(180);

/// How long a connection may stay silent: `limit` until its first message and inside a frame, and between messages
/// only when `heartbeats` (the duel connection).
#[derive(Clone, Copy, Debug)]
pub struct Idle {
    pub limit: Duration,
    pub heartbeats: bool,
}

/// Frame bodies are read in pieces of this size, so a frame's buffer grows only as its bytes really arrive.
const READ_CHUNK: usize = 16 * 1024;

/// Runs blocking work (a handler may wait on the rules engine) without holding up the runtime's other tasks.
pub(crate) fn blocking<R>(work: impl FnOnce() -> R) -> R {
    match tokio::runtime::Handle::try_current().map(|h| h.runtime_flavor()) {
        Ok(tokio::runtime::RuntimeFlavor::MultiThread) => tokio::task::block_in_place(work),
        _ => work(),
    }
}

/// Sends everything queued for one connection, in order, until every outbox clone is gone.
async fn write_loop<W: AsyncWrite + Unpin>(mut stream: W, mut queue: UnboundedReceiver<Message>, who: &str, log: Arc<PacketLog>, mut stop: tokio::sync::oneshot::Receiver<()>) {
    loop {
        let msg = tokio::select! { biased; msg = queue.recv() => msg, _ = &mut stop => None };
        let Some(msg) = msg else { break };
        let plain = msg.encode();
        log.outgoing(who, &plain, &msg);
        if stream.write_all(&frame(&plain)).await.is_err() {
            break;
        }
    }
    let _ = stream.shutdown().await;
}

async fn serve<R, H>(mut stream: R, who: &str, log: &PacketLog, idle: Idle, mut handler: H, outbox: Outbox) -> std::io::Result<()>
where
    R: AsyncRead + Unpin,
    H: FnMut(&Packet) -> Vec<Message>,
{
    let mut head = [0u8; 4];
    let mut spoken = false;
    loop {
        let read = if spoken && !idle.heartbeats {
            Ok(stream.read_exact(&mut head).await)
        } else {
            tokio::time::timeout(idle.limit, stream.read_exact(&mut head)).await
        };
        match read {
            Err(_) => {
                tracing::warn!("[{who}] idle for {:?}; closing", idle.limit);
                return Ok(());
            }
            Ok(Err(e)) => return if e.kind() == std::io::ErrorKind::UnexpectedEof { Ok(()) } else { Err(e) },
            Ok(Ok(_)) => {}
        }
        let n = frame_len(head);
        if n == 0 || n > MAX_FRAME {
            tracing::warn!("[{who}] bad frame length {n}; closing");
            return Ok(());
        }
        let mut body = Vec::with_capacity(n.min(READ_CHUNK));
        let mut chunk = [0u8; READ_CHUNK];
        while body.len() < n {
            let want = (n - body.len()).min(READ_CHUNK);
            match tokio::time::timeout(idle.limit, stream.read(&mut chunk[..want])).await {
                Err(_) => {
                    tracing::warn!("[{who}] stalled inside a frame for {:?}; closing", idle.limit);
                    return Ok(());
                }
                Ok(Ok(0)) => return Err(std::io::ErrorKind::UnexpectedEof.into()),
                Ok(Ok(k)) => body.extend_from_slice(&chunk[..k]),
                Ok(Err(e)) => return Err(e),
            }
        }
        let plain = match decrypt(&body) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("[{who}] {e}");
                continue;
            }
        };
        let packet = match Packet::decode(&plain) {
            Ok(p) => {
                log.incoming(who, &plain, Some(&p), None);
                // The opener alone doesn't count: a socket that sends it and then nothing is still closed.
                spoken |= matches!(p, Packet::Message(_));
                p
            }
            Err(e) => {
                log.incoming(who, &plain, None, Some(&e.to_string()));
                // Still let the handler answer (the first packet only needs to exist).
                Packet::Message(Message::default())
            }
        };
        let replies = blocking(|| handler(&packet));
        if replies.is_empty()
            && let Packet::Message(m) = &packet
            && let Some(id) = m.request()
            && id != crate::front::HEARTBEAT
            // The duel connection's 0x200C often needs no answer (the duel options {player, autoSet} on entering the
            // field, a pick with nothing asked): no warning, it only misled hosts reading the log (issue #2).
            && !(who == "duel" && id == 0x200C)
        {
            tracing::warn!("[{who}] UNHANDLED request {id:#x}");
        }
        for reply in replies {
            let _ = outbox.send(reply);
        }
    }
}

/// The same framing and session lifecycle on an authenticated TLS stream.
pub(crate) async fn secured<S, H, M>(stream: S, who: &'static str, log: Arc<PacketLog>, idle: Idle, make: M) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    H: FnMut(&Packet) -> Vec<Message>,
    M: FnOnce(Outbox) -> H,
{
    let (outbox, queue) = unbounded_channel();
    let handler = make(outbox.clone());
    let (reader, writer) = tokio::io::split(stream);
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let mut writer = tokio::spawn(write_loop(writer, queue, who, log.clone(), stopped));
    let result = serve(reader, who, &log, idle, handler, outbox).await;
    let _ = stop.send(());
    if tokio::time::timeout(std::time::Duration::from_secs(1), &mut writer).await.is_err() { writer.abort(); }
    result
}

/// Minimal HTTP: serves the client's own patch index so it sees itself as up to date.
///
/// This is an internal loopback asset/admin service. Account keys in old patch URLs are ignored; TLS identifies
/// accounts before the game protocol runs. The authenticated asset tunnel whitelists routes before reaching here.
pub async fn http_loop(
    listener: TcpListener,
    game_dir: PathBuf,
    duel_logs: Option<PathBuf>,
    accounts: Arc<Mutex<Directory>>,
    admin_token: Option<String>,
    manager: crate::manager::Manager,
) {
    let mut backoff = Duration::from_millis(50);
    loop {
        let (mut stream, peer) = match listener.accept().await {
            Ok(accepted) => {
                backoff = Duration::from_millis(50);
                accepted
            }
            Err(e) => {
                tracing::warn!("[http] accept failed: {e}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(2));
                continue;
            }
        };
        let (game_dir, duel_logs, accounts, admin_token) = (game_dir.clone(), duel_logs.clone(), accounts.clone(), admin_token.clone());
        let manager = manager.clone();
        tokio::spawn(async move {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 1024];
            while !buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() < 8192 {
                // A client that never finishes its request line is let go after a while.
                match tokio::time::timeout(Duration::from_secs(10), stream.read(&mut chunk)).await {
                    Ok(Ok(n)) if n > 0 => buf.extend_from_slice(&chunk[..n]),
                    _ => break,
                }
            }
            if buf.is_empty() { return } // A local liveness probe has no HTTP request to log.
            let header_end = buf.windows(4).position(|w| w == b"\r\n\r\n").map(|n| n + 4).unwrap_or(buf.len());
            let length = String::from_utf8_lossy(&buf[..header_end]).lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse::<usize>().ok()).flatten()
            }).unwrap_or(0);
            if length <= 8192 {
                while buf.len() < header_end.saturating_add(length) {
                    match tokio::time::timeout(Duration::from_secs(10), stream.read(&mut chunk)).await {
                        Ok(Ok(n)) if n > 0 => buf.extend_from_slice(&chunk[..n]), _ => break,
                    }
                }
            }
            let request = String::from_utf8_lossy(&buf);
            let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
            let response = if path == "/manager" {
                Some(manager.handle(&request, peer.ip(), admin_token.as_deref()))
            } else { crate::admin::handle(&path, peer.ip(), admin_token.as_deref(), &accounts) };
            if let Some(response) = response {
                if path != "/manager" {
                    tracing::info!("[http] GET {} -> {} ({} B)", path.split('?').next().unwrap_or_default(), response.status, response.body.len());
                }
                let mut head = format!("HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n", response.status, response.content_type, response.body.len());
                for (name, value) in &response.headers {
                    head += &format!("{name}: {value}\r\n");
                }
                head += "\r\n";
                let _ = stream.write_all(head.as_bytes()).await;
                let _ = stream.write_all(&response.body).await;
                let _ = stream.shutdown().await;
                return;
            }
            let (status, body) = if path.trim_end_matches('/').ends_with("/yo3.idx") {
                match tokio::fs::read(game_dir.join("yo3.idx")).await {
                    Ok(b) => ("200 OK", b),
                    Err(_) => ("404 Not Found", vec![]),
                }
            } else if let Some(file) = path.strip_prefix("/duellogs/").and_then(|n| crate::duellog::path_of(duel_logs.as_deref()?, n)) {
                match tokio::fs::read(file).await {
                    Ok(b) => ("200 OK", b),
                    Err(_) => ("404 Not Found", vec![]),
                }
            } else if path.starts_with("/app/") {
                ("200 OK", vec![])
            } else {
                ("404 Not Found", vec![])
            };
            tracing::info!("[http] GET {} -> {status} ({} B)", if path.ends_with("/yo3.idx") { "/yo3/yo3.idx" } else { path.split('?').next().unwrap_or_default() }, body.len());
            let head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = stream.write_all(head.as_bytes()).await;
            let _ = stream.write_all(&body).await;
            let _ = stream.shutdown().await;
        });
    }
}
