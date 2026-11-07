//! Connection loops: framing, decryption, logging, and the patch-HTTP endpoint.
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use yo3_proto::{MAX_FRAME, Message, Packet, decrypt, frame, frame_len};

use crate::front::Outbox;
use crate::log::PacketLog;

/// Accepts connections. `make` builds a fresh packet handler for each one; it gets the peer's address, the
/// address the peer reached (this end), and the connection's outbox, through which anything (replies, pushes
/// from other sessions) is sent.
pub async fn accept_loop<M, H>(listener: TcpListener, who: &'static str, log: Arc<PacketLog>, make: M)
where
    M: Fn(SocketAddr, SocketAddr, Outbox) -> H,
    H: FnMut(&Packet) -> Vec<Message> + Send + 'static,
{
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                tracing::info!("[{who}] connected {peer}");
                let (outbox, queue) = unbounded_channel();
                let local = stream.local_addr().unwrap_or(peer);
                let handler = make(peer, local, outbox.clone());
                let (reader, writer) = stream.into_split();
                tokio::spawn(write_loop(writer, queue, who, log.clone()));
                let log = log.clone();
                tokio::spawn(async move {
                    match serve(reader, who, &log, handler, outbox).await {
                        Ok(()) => tracing::info!("[{who}] {peer} closed"),
                        Err(e) => tracing::warn!("[{who}] {peer}: {e}"),
                    }
                });
            }
            Err(e) => tracing::warn!("[{who}] accept failed: {e}"),
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

async fn serve<H>(mut stream: OwnedReadHalf, who: &str, log: &PacketLog, mut handler: H, outbox: Outbox) -> std::io::Result<()>
where
    H: FnMut(&Packet) -> Vec<Message>,
{
    let mut head = [0u8; 4];
    loop {
        if let Err(e) = stream.read_exact(&mut head).await {
            return if e.kind() == std::io::ErrorKind::UnexpectedEof { Ok(()) } else { Err(e) };
        }
        let n = frame_len(head);
        if n == 0 || n > MAX_FRAME {
            tracing::warn!("[{who}] bad frame length {n}; closing");
            return Ok(());
        }
        let mut body = vec![0; n];
        stream.read_exact(&mut body).await?;
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
        let replies = handler(&packet);
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
pub async fn http_loop(listener: TcpListener, game_dir: PathBuf) {
    loop {
        let Ok((mut stream, _)) = listener.accept().await else { continue };
        let game_dir = game_dir.clone();
        tokio::spawn(async move {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 1024];
            while !buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() < 8192 {
                match stream.read(&mut chunk).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                }
            }
            let request = String::from_utf8_lossy(&buf);
            let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
            let (status, body) = if path.trim_end_matches('/').ends_with("/yo3.idx") {
                match tokio::fs::read(game_dir.join("yo3.idx")).await {
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
