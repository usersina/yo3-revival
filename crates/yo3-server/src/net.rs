//! Connection loops: framing, decryption, logging, and the patch-HTTP endpoint.
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use yo3_proto::{MAX_FRAME, Message, Packet, decrypt, frame, frame_len};

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

/// A connection's request pace and outbox size (live settings `request-burst`, `requests-per-second` and `outbox-max`,
/// read when the connection opens). Security review (#39).
#[derive(Clone, Copy, Debug)]
pub struct Pace {
    /// Requests a connection may make at once. A request costs one, plus one per [`PACE_BYTES`] of its frame.
    pub burst: u32,
    /// How fast the allowance comes back. Past it, the next request waits: it is paced, never dropped.
    pub per_second: u32,
    /// Messages that may wait for a client that isn't reading before its connection ends.
    pub outbox_max: usize,
}

impl Pace {
    /// Unlimited pace and outbox: for tests and tools that drive a connection as fast as they can.
    pub const UNLIMITED: Pace = Pace { burst: u32::MAX, per_second: u32::MAX, outbox_max: usize::MAX };
}

/// A request's frame costs one more request per this many bytes, so large frames use the allowance faster.
pub const PACE_BYTES: usize = 16 * 1024;

/// How long one write to a client may wait (the client reads nothing) before its connection ends.
pub const WRITE_STALL: Duration = Duration::from_secs(60);

/// A token bucket over virtual time: `take` waits until `cost` tokens are there.
struct Bucket { tokens: f64, at: tokio::time::Instant, pace: Pace }

impl Bucket {
    fn new(pace: Pace) -> Bucket { Bucket { tokens: f64::from(pace.burst), at: tokio::time::Instant::now(), pace } }
    async fn take(&mut self, cost: f64) {
        if self.pace.per_second == u32::MAX { return }
        let (burst, rate) = (f64::from(self.pace.burst), f64::from(self.pace.per_second.max(1)));
        let cost = cost.min(burst);
        let now = tokio::time::Instant::now();
        self.tokens = (self.tokens + now.duration_since(self.at).as_secs_f64() * rate).min(burst);
        self.at = now;
        if self.tokens < cost {
            let wait = Duration::from_secs_f64((cost - self.tokens) / rate);
            tokio::time::sleep(wait).await;
            self.tokens = cost;
            self.at = tokio::time::Instant::now();
        }
        self.tokens -= cost;
    }
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

/// Sends everything queued for one connection, in order, until every outbox clone is gone. Returns false when it
/// gave up on the client: a write failed, stalled for [`WRITE_STALL`], or more than `outbox_max` messages waited.
async fn write_loop<W: AsyncWrite + Unpin>(mut stream: W, mut queue: UnboundedReceiver<Message>, who: &str, log: Arc<PacketLog>,
    mut stop: tokio::sync::oneshot::Receiver<()>, outbox_max: usize) -> bool {
    let mut kept = true;
    'messages: loop {
        let msg = tokio::select! { biased; msg = queue.recv() => msg, _ = &mut stop => None };
        let Some(msg) = msg else { break };
        let plain = msg.encode();
        log.outgoing(who, &plain, &msg);
        let bytes = frame(&plain);
        let started = tokio::time::Instant::now();
        let mut check = tokio::time::interval(Duration::from_millis(250));
        let write = stream.write_all(&bytes);
        tokio::pin!(write);
        loop {
            tokio::select! {
                written = &mut write => if written.is_ok() { break } else { kept = false; break 'messages },
                _ = check.tick() => {
                    // The queue only grows while a write waits: a client reading nothing.
                    if queue.len() > outbox_max {
                        tracing::warn!("[{who}] {} messages wait for a client that isn't reading (outbox-max); disconnecting", queue.len());
                        kept = false; break 'messages
                    }
                    if started.elapsed() >= WRITE_STALL {
                        tracing::warn!("[{who}] the client read nothing for {WRITE_STALL:?}; disconnecting");
                        kept = false; break 'messages
                    }
                }
            }
        }
    }
    let _ = tokio::time::timeout(Duration::from_secs(1), stream.shutdown()).await;
    kept
}

async fn serve<R, H>(mut stream: R, who: &str, log: &PacketLog, idle: Idle, pace: Pace, mut handler: H, outbox: Outbox) -> std::io::Result<()>
where
    R: AsyncRead + Unpin,
    H: FnMut(&Packet) -> Vec<Message>,
{
    let mut head = [0u8; 4];
    let mut spoken = false;
    let mut allowance = Bucket::new(pace);
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
        // Past the burst a request waits for its allowance (security review, #39): a flood of requests that save, under
        // the hub lock, goes no faster than `requests-per-second`.
        allowance.take(1.0 + (n / PACE_BYTES) as f64).await;
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
            // Entering a duel between players (0x200A) is answered once the second player sits: the first waits.
            && !(who == "duel" && id == 0x200A)
            // Edit Deck's saves (0x1F4F without a duel_id, from the connection DLL) are kept unanswered on purpose.
            && !(who == "front" && id == 0x1F4F && m.get("duel_id").is_none())
            // An early rental return (issue #13) is answered by pushes.
            && !(who == "front" && id == crate::front::REVIVAL_RETURN)
        {
            tracing::warn!("[{who}] UNHANDLED request {id:#x}");
        }
        for reply in replies {
            let _ = outbox.send(reply);
        }
    }
}

/// The same framing and session lifecycle on an authenticated TLS stream. The session ends when the client does, or
/// when its writer gives up on a client that doesn't read.
pub(crate) async fn secured<S, H, M>(stream: S, who: &'static str, log: Arc<PacketLog>, idle: Idle, pace: Pace, make: M) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    H: FnMut(&Packet) -> Vec<Message>,
    M: FnOnce(Outbox) -> H,
{
    let (outbox, queue) = unbounded_channel();
    let handler = make(outbox.clone());
    let (reader, writer) = tokio::io::split(stream);
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let mut writer = tokio::spawn(write_loop(writer, queue, who, log.clone(), stopped, pace.outbox_max));
    let served = tokio::select! {
        result = serve(reader, who, &log, idle, pace, handler, outbox) => Some(result),
        // The writer gave up (or failed): the session ends here, its handler (and the player's place) with it.
        _ = &mut writer => None,
    };
    let Some(result) = served else { return Ok(()) };
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
    data_dir: Option<PathBuf>,
    admin_token: Option<String>,
    api: Arc<crate::api::Api>,
    panel: Arc<crate::panel::Panel>,
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
        let (game_dir, data_dir, admin_token) = (game_dir.clone(), data_dir.clone(), admin_token.clone());
        let (api, panel) = (api.clone(), panel.clone());
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
            // The admin panel's forms (news) can be larger than a game request; nothing public posts here (transport.rs).
            if length <= 64 * 1024 {
                while buf.len() < header_end.saturating_add(length) {
                    match tokio::time::timeout(Duration::from_secs(10), stream.read(&mut chunk)).await {
                        Ok(Ok(n)) if n > 0 => buf.extend_from_slice(&chunk[..n]), _ => break,
                    }
                }
            }
            let request = String::from_utf8_lossy(&buf);
            let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
            // The admin API (#25); the old /admin and /manager routes are gone.
            let mut response = crate::api::handle(&request, peer.ip(), admin_token.as_deref(), &api);
            // The monitor's health check (#37) and the admin panel (#36).
            if response.is_none() && request.starts_with("GET /health ") && peer.ip().is_loopback() { response = Some(crate::api::health(&api)) }
            if response.is_none() { response = crate::panel::handle(&request, peer.ip(), &panel, &api).await }
            if let Some(response) = response {
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
            } else if let (Some(name), Some(dir)) = (path.strip_prefix("/duellogs/").filter(|n| crate::duellog::valid_name(n)), data_dir.clone()) {
                let name = name.to_string();
                match tokio::task::spawn_blocking(move || crate::store::duel_log(&dir, &name)).await {
                    Ok(Ok(Some(b))) => ("200 OK", b),
                    _ => ("404 Not Found", vec![]),
                }
            } else if path.starts_with("/app/") {
                ("200 OK", vec![])
            } else if let Some(old) = path.strip_prefix("/web") {
                // The game's links to Konami's old website (crate::web).
                ("200 OK", crate::web::page(old).into_bytes())
            } else {
                ("404 Not Found", vec![])
            };
            tracing::info!("[http] GET {} -> {status} ({} B)", if path.ends_with("/yo3.idx") { "/yo3/yo3.idx" } else { path.split('?').next().unwrap_or_default() }, body.len());
            let kind = if path.starts_with("/web") { "Content-Type: text/html; charset=utf-8\r\n" } else { "" };
            let head = format!("HTTP/1.1 {status}\r\n{kind}Content-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = stream.write_all(head.as_bytes()).await;
            let _ = stream.write_all(&body).await;
            let _ = stream.shutdown().await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heartbeat() -> Vec<u8> { frame(&Message::reply(crate::front::HEARTBEAT).encode()) }

    #[tokio::test(start_paused = true)]
    async fn a_connection_past_its_burst_is_paced_not_cut() {
        // Security review (#39): requests that save hold the hub lock. Past the burst, a connection's requests wait for
        // the refill instead of being answered at once; nothing is dropped. Real play peaks near 32 a second on the front
        // and 6 on the duel connection, in a burst well under the default 200 (packet logs, 2026-11-07..09).
        let (mut client, server) = tokio::io::duplex(1 << 20);
        let log = Arc::new(PacketLog::disabled().unwrap());
        let pace = Pace { burst: 5, per_second: 10, outbox_max: 1000 };
        let idle = Idle { limit: IDLE_TIMEOUT, heartbeats: false };
        let handled = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = handled.clone();
        let start = tokio::time::Instant::now();
        let session = tokio::spawn(secured(server, "front", log, idle, pace, move |_| move |_: &Packet| {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst); vec![]
        }));
        for _ in 0..25 { client.write_all(&heartbeat()).await.unwrap(); }
        while handled.load(std::sync::atomic::Ordering::SeqCst) < 25 { tokio::time::sleep(Duration::from_millis(10)).await; }
        let took = start.elapsed();
        assert!(took >= Duration::from_millis(1900) && took < Duration::from_secs(3), "20 past a burst of 5 at 10/s: {took:?}");
        drop(client);
        session.await.unwrap().unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn a_client_that_does_not_read_is_disconnected_when_its_outbox_fills() {
        // A player who never reads their socket: pushes queue behind a write that can't finish. Past `outbox_max`
        // waiting messages the connection ends, and the queue with it.
        let (client, server) = tokio::io::duplex(64);
        let log = Arc::new(PacketLog::disabled().unwrap());
        let pace = Pace { burst: 200, per_second: 20, outbox_max: 100 };
        let idle = Idle { limit: IDLE_TIMEOUT, heartbeats: false };
        let (mut read_half, mut write_half) = tokio::io::split(client);
        let session = tokio::spawn(secured(server, "front", log, idle, pace, move |outbox: Outbox| move |_: &Packet| {
            for _ in 0..500 { let _ = outbox.send(Message::reply(0x21FC).with("news", yo3_proto::Value::text(&"x".repeat(100)))); }
            vec![]
        }));
        write_half.write_all(&heartbeat()).await.unwrap();
        let ended = tokio::time::timeout(Duration::from_secs(5), session).await;
        assert!(matches!(ended, Ok(Ok(Ok(())))), "the session ends once 100 messages wait: {ended:?}");
        let _ = read_half.read(&mut [0; 1]).await;
    }

    #[tokio::test(start_paused = true)]
    async fn a_write_stalled_for_a_minute_ends_the_connection() {
        // A few messages never read for WRITE_STALL also end it (the queue stays under the cap).
        let (client, server) = tokio::io::duplex(64);
        let log = Arc::new(PacketLog::disabled().unwrap());
        let pace = Pace { burst: 200, per_second: 20, outbox_max: 1000 };
        let idle = Idle { limit: IDLE_TIMEOUT, heartbeats: false };
        let (_read_half, mut write_half) = tokio::io::split(client);
        let start = tokio::time::Instant::now();
        let session = tokio::spawn(secured(server, "front", log, idle, pace, move |outbox: Outbox| move |_: &Packet| {
            for _ in 0..5 { let _ = outbox.send(Message::reply(0x21FC).with("news", yo3_proto::Value::text(&"x".repeat(100)))); }
            vec![]
        }));
        write_half.write_all(&heartbeat()).await.unwrap();
        let ended = tokio::time::timeout(Duration::from_secs(120), session).await;
        assert!(matches!(ended, Ok(Ok(Ok(())))), "{ended:?}");
        assert!(start.elapsed() >= WRITE_STALL, "{:?}", start.elapsed());
    }
}
