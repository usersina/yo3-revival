//! Connection loops: framing, decryption, logging, and the patch-HTTP endpoint.
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use yo3_proto::{MAX_FRAME, Message, Packet, decrypt, frame, frame_len};

use crate::front::Outbox;
use crate::log::PacketLog;

/// How long a connection may stay silent before it is closed. The real client heartbeats every 30 s (`0x2724`) on the
/// front and the duel connection, so this is generous; the gate's only message comes at once.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(180);

/// Frame bodies are read in pieces of this size, so a frame's buffer grows only as its bytes really arrive.
const READ_CHUNK: usize = 16 * 1024;

/// Runs blocking work (a handler may wait on the rules engine) without holding up the runtime's other tasks.
pub(crate) fn blocking<R>(work: impl FnOnce() -> R) -> R {
    match tokio::runtime::Handle::try_current().map(|h| h.runtime_flavor()) {
        Ok(tokio::runtime::RuntimeFlavor::MultiThread) => tokio::task::block_in_place(work),
        _ => work(),
    }
}

/// Accepts connections. `make` builds a fresh packet handler for each one; it gets the peer's address, the
/// address the peer reached (this end), and the connection's outbox, through which anything (replies, pushes
/// from other sessions) is sent.
pub async fn accept_loop<M, H>(listener: TcpListener, who: &'static str, log: Arc<PacketLog>, idle: Duration, make: M)
where
    M: Fn(SocketAddr, SocketAddr, Outbox) -> H,
    H: FnMut(&Packet) -> Vec<Message> + Send + 'static,
{
    let mut backoff = Duration::from_millis(50);
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                backoff = Duration::from_millis(50);
                tracing::info!("[{who}] connected {peer}");
                let (outbox, queue) = unbounded_channel();
                let local = stream.local_addr().unwrap_or(peer);
                let handler = make(peer, local, outbox.clone());
                let (reader, writer) = stream.into_split();
                tokio::spawn(write_loop(writer, queue, who, log.clone()));
                let log = log.clone();
                tokio::spawn(async move {
                    match serve(reader, who, &log, idle, handler, outbox).await {
                        Ok(()) => tracing::info!("[{who}] {peer} closed"),
                        Err(e) => tracing::warn!("[{who}] {peer}: {e}"),
                    }
                });
            }
            Err(e) => {
                // A persistent error (out of file descriptors) would otherwise spin this loop at full speed.
                tracing::warn!("[{who}] accept failed: {e}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(2));
            }
        }
    }
}

/// Sends everything queued for one connection, in order, until every outbox clone is gone.
async fn write_loop(mut stream: OwnedWriteHalf, mut queue: UnboundedReceiver<Message>, who: &str, log: Arc<PacketLog>) {
    while let Some(msg) = queue.recv().await {
        let plain = msg.encode();
        log.outgoing(who, &plain, &msg);
        if stream.write_all(&frame(&plain)).await.is_err() {
            break;
        }
    }
}

async fn serve<H>(mut stream: OwnedReadHalf, who: &str, log: &PacketLog, idle: Duration, mut handler: H, outbox: Outbox) -> std::io::Result<()>
where
    H: FnMut(&Packet) -> Vec<Message>,
{
    let mut head = [0u8; 4];
    loop {
        match tokio::time::timeout(idle, stream.read_exact(&mut head)).await {
            Err(_) => {
                tracing::warn!("[{who}] idle for {idle:?}; closing");
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
            match tokio::time::timeout(idle, stream.read(&mut chunk[..want])).await {
                Err(_) => {
                    tracing::warn!("[{who}] stalled inside a frame for {idle:?}; closing");
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
        {
            tracing::warn!("[{who}] UNHANDLED request {id:#x}");
        }
        for reply in replies {
            let _ = outbox.send(reply);
        }
    }
}

/// Minimal HTTP: serves the client's own patch index so it sees itself as up to date.
pub async fn http_loop(listener: TcpListener, game_dir: PathBuf, duel_logs: Option<PathBuf>) {
    let mut backoff = Duration::from_millis(50);
    loop {
        let (mut stream, _) = match listener.accept().await {
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
        let (game_dir, duel_logs) = (game_dir.clone(), duel_logs.clone());
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
            let request = String::from_utf8_lossy(&buf);
            let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
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
            tracing::info!("[http] GET {path} -> {status} ({} B)", body.len());
            let head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = stream.write_all(head.as_bytes()).await;
            let _ = stream.write_all(&body).await;
            let _ = stream.shutdown().await;
        });
    }
}
