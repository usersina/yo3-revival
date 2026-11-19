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
pub mod runtime;
pub mod resolution;
pub mod direct;
pub mod vault;
pub mod signin;
pub mod survey;

pub async fn authorize<R: AsyncRead + Unpin>(stream: &mut R, cookie: &Token) -> anyhow::Result<()> {
    let mut given = [0; 32];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut given)).await??;
    anyhow::ensure!(bool::from(given.ct_eq(cookie)), "local connection refused");
    Ok(())
}

#[derive(Deserialize)]
pub struct Profile { pub server: String, pub server_name: String, #[serde(default)] pub certificate: String,
    #[serde(default)] pub identity: Option<String>, #[serde(default)] pub key: String,
    /// A session the launcher signed in for with a User ID and password (#30), instead of an account key.
    #[serde(default)] pub session: Option<String>,
    /// The vault entry of this device's saved sign-in for the server ("Stay signed in"): direct start resumes it when
    /// there is neither a key nor a session, and Delete Login Data forgets it (#33).
    #[serde(default)] pub vault: Option<String>,
    /// The credential store's entry that holds the account key (Singleplayer, #23), when the profile doesn't hold it.
    #[serde(default)] pub key_vault: Option<String>,
    /// Singleplayer's direct start: how to start your own server when it isn't running (#33).
    #[serde(default)] pub start: Option<direct::ServerStart> }

/// A profile, read.
pub struct Loaded { pub endpoint: Endpoint, pub credential: Credential, pub vault: Option<String>, pub start: Option<direct::ServerStart> }

/// How the helper signs in: with an account key (key accounts: Singleplayer, the Scripts profiles), or with a session
/// the launcher already signed in for (password accounts, #30), or with the device's saved sign-in (direct start, #33).
#[derive(Clone)]
pub enum Credential { Key(String), Session(yo3_tunnel::Token), Device(String) }

impl Credential {
    /// A session for the game's connections.
    pub async fn sign_in(&self, endpoint: &Endpoint) -> anyhow::Result<yo3_tunnel::Token> {
        let read = |account: &str| vault::Vault::get(&vault::SystemVault, account).ok().flatten();
        direct::sign_in(endpoint, self, &read).await.map_err(|failure| anyhow::anyhow!("{}", failure.reason))
    }
    /// Whether the helper ends the session when the game exits: only one it signed in itself (a key). A session the
    /// launcher handed over is the launcher's own sign-in, reused by its next Play; its Sign out ends it.
    pub fn owns_session(&self) -> bool { matches!(self, Credential::Key(_) | Credential::Device(_)) }
    /// The secret, for redacting logs.
    pub fn secret(&self) -> String {
        match self { Credential::Key(key) => key.clone(), Credential::Session(session) => hex(session), Credential::Device(_) => String::new() }
    }
}

impl Profile {
    pub fn load(path: &Path) -> anyhow::Result<(Endpoint, Credential)> {
        let loaded = Self::read(path)?; Ok((loaded.endpoint, loaded.credential))
    }
    pub fn read(path: &Path) -> anyhow::Result<Loaded> { Self::read_with(path, &vault::SystemVault) }
    /// Reads a profile whose key may be in `vault` (`key_vault`).
    pub fn read_with(path: &Path, store: &dyn vault::Vault) -> anyhow::Result<Loaded> {
        let mut p: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        if p.key.is_empty() && p.session.is_none() && let Some(entry) = &p.key_vault {
            p.key = store.get(entry)?.ok_or_else(|| anyhow::anyhow!("your server's account key isn't in the system's credential store: open YO3 Revival and press Play once"))?;
        }
        let certificate = path.parent().unwrap_or(Path::new(".")).join(p.certificate);
        let endpoint = match p.identity { Some(identity) => Endpoint::pinned(p.server, p.server_name, &identity)?,
            None => Endpoint::new(p.server, p.server_name, &certificate)? };
        let credential = match p.session {
            Some(hex) => {
                let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()).collect();
                Credential::Session(bytes.try_into().map_err(|_| anyhow::anyhow!("the profile's session is malformed"))?)
            }
            None if p.key.is_empty() && p.vault.is_some() => Credential::Device(p.vault.clone().unwrap()),
            None => Credential::Key(p.key),
        };
        Ok(Loaded { endpoint, credential, vault: p.vault, start: p.start })
    }
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct Ports { pub gate: u16, pub front: u16, pub duel: u16, pub http: u16 }

/// How the launcher checks the server while the game is connected to it. A server process that hangs (stopped, or
/// stuck on a lock) keeps its connections open and its machine answers TCP keepalive, so the game waited in its duel
/// for good with no message. While the game has a front or duel connection, the launcher asks the server's discovery
/// (the server list's request, which also takes the server's hub lock) every `every`; after `misses` in a row get no
/// answer within `timeout`, it closes the game's connections, and the game shows its own "disconnected" dialogs and
/// returns to the title. Old servers need no change.
#[derive(Clone, Copy, Debug)]
pub struct Liveness { pub every: Duration, pub timeout: Duration, pub misses: u32 }
impl Default for Liveness {
    /// At most about 75 s from the server's last answer to the game's disconnect.
    fn default() -> Self { Self { every: Duration::from_secs(15), timeout: Duration::from_secs(10), misses: 3 } }
}

/// How the game's sign-in ended: a session for its connections, or a refusal its login dialog shows (#33).
#[derive(Clone, Copy, Debug)]
pub enum Outcome { Session(Token), Refused(direct::Refusal) }

pub struct Proxy { pub ports: Ports, pub cookie: Token, pub server_id: String, tasks: Vec<JoinHandle<()>>,
    session: tokio::sync::watch::Sender<Option<Outcome>>, sign_outs: std::sync::Arc<tokio::sync::Notify>,
    typed: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<direct::Typed>>>, form_again: std::sync::Arc<std::sync::atomic::AtomicBool>,
    index: LocalIndex }

/// The game's own patch index (`yo3.idx` beside `yo3.exe`), the only answer the game's patch check ever gets.
type LocalIndex = std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>;
/// The patch index's path under the launch cookie (the connection DLL maps the game's request to it, tools/mod/auth.c).
const INDEX_PATH: &str = "yo3/yo3.idx";

impl Drop for Proxy { fn drop(&mut self) { for task in &self.tasks { task.abort(); } } }

impl Proxy {
    pub async fn start(endpoint: Endpoint, session: Token, ports: Ports) -> anyhow::Result<Self> {
        let proxy = Self::prepare(endpoint, ports).await?; proxy.activate(session); Ok(proxy)
    }
    pub fn activate(&self, session: Token) { self.session.send_replace(Some(Outcome::Session(session))); }
    /// The sign-in failed: the game's login gets `refusal` from the front (and the duel and HTTP ports nothing).
    pub fn refuse(&self, refusal: direct::Refusal) { self.session.send_replace(Some(Outcome::Refused(refusal))); }
    /// A sign-in from the game's own form was refused: its next ONLINE shows the form again (#34).
    pub fn refuse_typed(&self, refusal: direct::Refusal) {
        self.form_again.store(true, std::sync::atomic::Ordering::SeqCst); self.refuse(refusal);
    }
    /// A sign-in from the form worked: until it ends, ONLINE goes straight on.
    pub fn activate_typed(&self, session: Token) {
        self.form_again.store(false, std::sync::atomic::Ordering::SeqCst); self.activate(session);
    }
    /// The game's own patch index: the game's startup patch check gets it (nothing to update) and goes on to its title
    /// and login, also while the sign-in is refused (the form or the refusal shows there). Without it the game says it
    /// needs an Internet connection and closes. No server's index is ever relayed (#39).
    pub fn local_index(&self, bytes: Vec<u8>) { *self.index.lock().unwrap() = Some(bytes); }
    /// The game folder's own `yo3.idx` as [`Self::local_index`], if it is there (at most 1 MiB).
    pub fn local_index_from(&self, game_dir: &std::path::Path) {
        if let Ok(bytes) = std::fs::read(game_dir.join("yo3.idx")) && bytes.len() <= 1 << 20 { self.local_index(bytes) }
    }
    /// Signalled when the game's Delete Login Data asks to forget this device's sign-in (tools/mod/signout.h).
    pub fn sign_outs(&self) -> std::sync::Arc<tokio::sync::Notify> { self.sign_outs.clone() }
    /// What the player typed in the game's own login form (#34), once: the DLL hands it over when the game connects to
    /// the front. The outcome is cleared as it arrives, so the game's login waits for this sign-in.
    pub fn typed_logins(&self) -> Option<tokio::sync::mpsc::UnboundedReceiver<direct::Typed>> { self.typed.lock().unwrap().take() }
    pub async fn prepare(endpoint: Endpoint, ports: Ports) -> anyhow::Result<Self> {
        Self::prepare_watched(endpoint, ports, Liveness::default()).await
    }
    /// [`Self::prepare`], checking the server as `liveness` says.
    pub async fn prepare_watched(endpoint: Endpoint, ports: Ports, liveness: Liveness) -> anyhow::Result<Self> {
        let (active, _) = tokio::sync::watch::channel(None);
        let gate = TcpListener::bind((Ipv4Addr::LOCALHOST, ports.gate)).await?;
        let front = TcpListener::bind((Ipv4Addr::LOCALHOST, ports.front)).await?;
        let duel = TcpListener::bind((Ipv4Addr::LOCALHOST, ports.duel)).await?;
        let http = TcpListener::bind((Ipv4Addr::LOCALHOST, ports.http)).await?;
        let ports = Ports { gate: gate.local_addr()?.port(), front: front.local_addr()?.port(), duel: duel.local_addr()?.port(), http: http.local_addr()?.port() };
        let mut cookie = [0; 32]; getrandom::fill(&mut cookie)?;
        let server_id = endpoint.server_id();
        let sign_outs = std::sync::Arc::new(tokio::sync::Notify::new());
        let (typed_tx, typed_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut tasks = vec![];
        tasks.push(tokio::spawn(async move {
            // Keep accepted gates owned by this task so dropping the proxy also closes idle redirects.
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    Some(_) = connections.join_next() => {},
                    accepted = gate.accept() => {
                        let Ok((mut s, _)) = accepted else { break };
                        connections.spawn(async move {
                            let _ = async {
                                authorize(&mut s, &cookie).await?;
                                let mut head = [0; 4]; s.read_exact(&mut head).await?;
                                let n = frame_len(head); anyhow::ensure!(n > 0 && n <= MAX_FRAME, "bad gate frame");
                                let mut data = vec![0; n]; s.read_exact(&mut data).await?;
                                let reply = Message::reply(0x21FE).with("host", Value::text("127.0.0.1")).with("port", Value::U32(ports.front as u32));
                                s.write_all(&frame(&reply.encode())).await?;
                                // The native window owns the gate until it leaves player selection. Premature EOF can
                                // hit its null error logger (0x535210) before it processes the redirect/disconnect.
                                tokio::io::copy(&mut s, &mut tokio::io::sink()).await?;
                                Ok::<_, anyhow::Error>(())
                            }.await;
                        });
                    },
                }
            }
        }));
        // The game's open front and duel connections, and which server loss they belong to: each loss moves it on and
        // closes the connections made before it.
        let relays = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (lost, _) = tokio::sync::watch::channel(0u64);
        let lost = std::sync::Arc::new(lost);
        tasks.push(tokio::spawn(watch_server(endpoint.clone(), liveness, relays.clone(), lost.clone())));
        for (listener, role) in [(front, Role::Front), (duel, Role::Duel)] {
            let endpoint = endpoint.clone();
            let active = active.clone();
            let (relays, lost) = (relays.clone(), lost.clone());
            tasks.push(tokio::spawn(async move {
                while let Ok((mut local, _)) = listener.accept().await {
                    let endpoint = endpoint.clone();
                    let mut active = active.subscribe();
                    let (relays, mut lost) = (relays.clone(), lost.subscribe());
                    tokio::spawn(async move {
                        let result = async {
                            authorize(&mut local, &cookie).await?;
                            let session = match wait_for_outcome(&mut active).await? {
                                Outcome::Session(session) => session,
                                Outcome::Refused(refusal) if role == Role::Front => return refuse_login(local, refusal).await,
                                Outcome::Refused(_) => anyhow::bail!("the sign-in was refused"),
                            };
                            let ticket = endpoint.ticket(&session, role).await?;
                            let mut remote = endpoint.connect(&ticket, role, ports.duel).await?;
                            lost.mark_unchanged();
                            let _open = Open::count(&relays);
                            tokio::select! {
                                relayed = relay(&mut local, &mut remote) => { relayed?; }
                                _ = lost.changed() => anyhow::bail!("the server stopped answering; the game's connection is closed"),
                            }
                            Ok::<_, anyhow::Error>(())
                        }.await;
                        if let Err(e) = result { let message = format!("{role:?} connection: {e:#}"); yo3_tunnel::diagnostics::event(&message); eprintln!("{message}"); }
                    });
                }
            }));
        }
        let http_active = active.clone();
        let form_again = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let index = LocalIndex::default();
        let control = Control { sign_outs: sign_outs.clone(), typed: typed_tx, outcome: active.clone(), form_again: form_again.clone(), index: index.clone() };
        tasks.push(tokio::spawn(async move {
            while let Ok((local, _)) = http.accept().await {
                let endpoint = endpoint.clone();
                let active = http_active.subscribe();
                let control = control.clone();
                tokio::spawn(async move { let _ = relay_http(local, &endpoint, active, &cookie, &control).await; });
            }
        }));
        Ok(Self { ports, cookie, server_id, tasks, session: active, sign_outs, typed: std::sync::Mutex::new(Some(typed_rx)), form_again, index })
    }

    pub fn environment(&self) -> Vec<(&'static str, String)> {
        [("YO3_LOCAL_GATE", self.ports.gate), ("YO3_LOCAL_FRONT", self.ports.front), ("YO3_LOCAL_DUEL", self.ports.duel), ("YO3_LOCAL_HTTP", self.ports.http)]
            .into_iter().map(|(k, v)| (k, v.to_string())).chain([("YO3_LOCAL_COOKIE", hex(&self.cookie)), ("YO3_SERVER_ID", self.server_id.clone())])
            .collect()
    }
}

/// How long the game's connections wait for the sign-in: direct start may be starting your own server meanwhile.
const SIGN_IN_WAIT: Duration = Duration::from_secs(60);

/// One open relay of the game's, counted while it lives.
struct Open(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl Open {
    fn count(relays: &std::sync::Arc<std::sync::atomic::AtomicUsize>) -> Self {
        relays.fetch_add(1, std::sync::atomic::Ordering::SeqCst); Self(relays.clone())
    }
}
impl Drop for Open { fn drop(&mut self) { self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst); } }

/// Carries a game connection both ways until both sides finish, as `copy_bidirectional` does, but an error says which
/// side failed: the game's local connection or the one to the server (#65: a bare Windows 10053 left that open).
async fn relay<G, S>(game: &mut G, server: &mut S) -> anyhow::Result<()>
where G: AsyncRead + tokio::io::AsyncWrite + Unpin, S: AsyncRead + tokio::io::AsyncWrite + Unpin {
    const GAME: &str = "the game's connection";
    const SERVER: &str = "the connection to the server";
    async fn pump(from: &mut (impl AsyncRead + Unpin), to: &mut (impl tokio::io::AsyncWrite + Unpin), source: &'static str,
                  target: &'static str) -> anyhow::Result<()> {
        use anyhow::Context;
        let mut buffer = vec![0; 16 * 1024];
        loop {
            let n = match from.read(&mut buffer).await {
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                result => result.context(source)?,
            };
            if n == 0 {
                // One side finished: tell the other, as copy_bidirectional does, and keep the other direction going.
                to.shutdown().await.context(target)?;
                return Ok(());
            }
            to.write_all(&buffer[..n]).await.context(target)?;
            to.flush().await.context(target)?;
        }
    }
    let (mut game_read, mut game_write) = tokio::io::split(game);
    let (mut server_read, mut server_write) = tokio::io::split(server);
    tokio::try_join!(pump(&mut game_read, &mut server_write, GAME, SERVER), pump(&mut server_read, &mut game_write, SERVER, GAME))?;
    Ok(())
}

/// [`Liveness`]: while the game has a connection open, asks the server's discovery; `misses` unanswered in a row
/// move `lost` on, which closes the game's connections.
async fn watch_server(endpoint: Endpoint, liveness: Liveness, relays: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    lost: std::sync::Arc<tokio::sync::watch::Sender<u64>>) {
    let mut missed = 0;
    loop {
        tokio::time::sleep(liveness.every).await;
        if relays.load(std::sync::atomic::Ordering::SeqCst) == 0 { missed = 0; continue }
        match tokio::time::timeout(liveness.timeout, endpoint.info()).await {
            Ok(Ok(_)) => missed = 0,
            _ => {
                missed += 1;
                if missed >= liveness.misses {
                    missed = 0;
                    let message = format!("the server did not answer {} checks in a row; closing the game's connections", liveness.misses);
                    yo3_tunnel::diagnostics::event(&message); eprintln!("{message}");
                    lost.send_modify(|n| *n += 1);
                }
            }
        }
    }
}

async fn wait_for_outcome(active: &mut tokio::sync::watch::Receiver<Option<Outcome>>) -> anyhow::Result<Outcome> {
    let outcome = *tokio::time::timeout(SIGN_IN_WAIT, active.wait_for(|outcome| outcome.is_some())).await??;
    outcome.ok_or_else(|| anyhow::anyhow!("launcher session is not ready"))
}

/// The front of a refused sign-in, answered here: the version opener gets `0x219F` and the game's login (`0x1F47`) the
/// refusal, which LoginWindow `0x469C00` shows in its error dialog `0x46D050` before going back to the title screen.
/// The socket stays open until the game closes it, like the gate's (an early end reaches the null logger `0x535210`).
async fn refuse_login(mut local: TcpStream, refusal: direct::Refusal) -> anyhow::Result<()> {
    let mut told = false;
    loop {
        let mut head = [0; 4];
        if local.read_exact(&mut head).await.is_err() { return Ok(()) }
        let n = frame_len(head); anyhow::ensure!(n > 0 && n <= MAX_FRAME, "bad front frame");
        let mut data = vec![0; n]; local.read_exact(&mut data).await?;
        let reply = match yo3_proto::Packet::decode(&yo3_proto::decrypt(&data)?)? {
            yo3_proto::Packet::Version(_) => Message::reply(0x219F),
            yo3_proto::Packet::Message(m) if m.request() == Some(0x1F47) => Message::reply(refusal.reply()),
            yo3_proto::Packet::Message(_) => continue,
        };
        local.write_all(&frame(&reply.encode())).await?;
        if reply.request() == Some(refusal.reply()) && !told {
            told = true;
            if let Some(line) = refusal.notice() { direct::notify(line) }
        }
    }
}

pub fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }

/// How the desktop's control pipe ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopStop {
    /// The desktop asked for the game to stop (Close game, Cancel, quitting with the game open).
    Asked,
    /// The pipe closed: the desktop exited or crashed. A game still starting is cancelled; a game being played keeps
    /// running, and its session ends with it (#29).
    Gone,
}

/// The desktop owns this pipe: a stop command, or its end, decides what happens to the authenticated game session.
pub async fn wait_for_desktop_stop(mut input: impl std::io::BufRead + Send + 'static) -> DesktopStop {
    let (done, stopped) = tokio::sync::oneshot::channel();
    // Tokio's stdin uses a blocking runtime task that cannot be cancelled. A detached OS thread instead lets
    // the launcher finish naturally even while the desktop still holds its input pipe open.
    std::thread::spawn(move || {
        let mut line = String::new();
        let how = loop {
            line.clear();
            match input.read_line(&mut line) {
                Ok(0) | Err(_) => break DesktopStop::Gone,
                Ok(_) if line.trim() == "stop" => break DesktopStop::Asked,
                Ok(_) => {},
            }
        };
        let _ = done.send(how);
    });
    stopped.await.unwrap_or(DesktopStop::Gone)
}

/// The path the connection DLL asks on Delete Login Data (tools/mod/signout.h), under the launch cookie.
pub const SIGN_OUT_PATH: &str = "revival/sign-out";
/// Whether the game shows its own login form (#34, tools/mod/login.h): "form" when there is no saved sign-in, else
/// "ready". It waits for the helper's first outcome.
pub const LOGIN_STATE_PATH: &str = "revival/login-state";
/// What the player typed in that form: a POST with a JSON body {user, password, stay}.
pub const LOGIN_PATH: &str = "revival/login";

/// The HTTP port's own requests from the connection DLL.
#[derive(Clone)]
struct Control { sign_outs: std::sync::Arc<tokio::sync::Notify>, typed: tokio::sync::mpsc::UnboundedSender<direct::Typed>,
    outcome: tokio::sync::watch::Sender<Option<Outcome>>, form_again: std::sync::Arc<std::sync::atomic::AtomicBool>, index: LocalIndex }

/// The most of a server's web page the proxy hands the browser.
const WEB_PAGE_MAX: u64 = 1 << 20;

/// A server's page behind the game's old website links (`web/`, opened in the player's browser on this loopback
/// origin), from the server's whole answer: its status and body as HTML in a sandbox, so no script, form or plugin of
/// a hostile server runs here, next to the launch cookie in the address (security review, #39).
fn web_page(answer: &[u8]) -> Vec<u8> {
    let (head, body) = match answer.windows(4).position(|w| w == b"\r\n\r\n") {
        Some(at) => (&answer[..at], &answer[at + 4..]),
        None => (answer, &[][..]),
    };
    let status = std::str::from_utf8(head).ok().and_then(|h| h.lines().next()).and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse::<u16>().ok()).filter(|c| (200..600).contains(c)).unwrap_or(502);
    let mut page = format!("HTTP/1.1 {status} {}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
        Content-Security-Policy: sandbox; default-src 'none'; style-src 'unsafe-inline'; img-src data:\r\n\
        X-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        if status < 300 { "OK" } else { "Error" }, body.len()).into_bytes();
    page.extend_from_slice(body);
    page
}

/// The paths the proxy carries to the server: the game's old website links (`web/`), duel logs (`duellogs/`) and its
/// info URL (`app/`). The patch index and the patch files never go there: the stock patcher writes what a patch
/// archive holds into the game folder under the archive's own names and replaces `yo3.exe` itself (PatchComponent
/// `0x4EA470`, `0x4EAA50`), so a server's answer could put any file there (docs/client-internals.md, "The patcher").
fn relayed(path: &str) -> bool { ["web/", "duellogs/", "app/"].iter().any(|p| path.starts_with(p)) }

/// The browser/WinINet uses the launch cookie in a local path, never an account credential in a remote URL.
async fn relay_http(mut local: TcpStream, endpoint: &Endpoint, mut active: tokio::sync::watch::Receiver<Option<Outcome>>, cookie: &Token,
    control: &Control) -> anyhow::Result<()> {
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
    if let Some(path) = path.strip_prefix(&prefix).filter(|p| text.starts_with("POST ") && *p == LOGIN_PATH) {
        let _ = path;
        // The password travels only on this loopback socket, under the launch cookie, and is never logged.
        let length: usize = text.lines().find_map(|l| l.split_once(':').filter(|(k, _)| k.eq_ignore_ascii_case("content-length")).map(|(_, v)| v.trim().to_owned()))
            .and_then(|v| v.parse().ok()).filter(|n| *n > 0 && *n <= 4096).ok_or_else(|| anyhow::anyhow!("bad login request"))?;
        let mut body = vec![0; length];
        tokio::time::timeout(Duration::from_secs(5), local.read_exact(&mut body)).await??;
        let typed: direct::Typed = serde_json::from_slice(&body)?;
        anyhow::ensure!(!typed.user.is_empty() && !typed.password.is_empty(), "empty login");
        control.outcome.send_replace(None);
        control.typed.send(typed)?;
        local.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n").await?;
        return Ok(local.shutdown().await?)
    }
    let path = path.strip_prefix(&prefix).filter(|_| text.starts_with("GET ")).ok_or_else(|| anyhow::anyhow!("local HTTP refused"))?;
    if path == LOGIN_STATE_PATH {
        let form = tokio::time::timeout(Duration::from_secs(15), active.wait_for(|o| o.is_some())).await.ok().and_then(Result::ok)
            .is_some_and(|o| match *o {
                Some(Outcome::Refused(direct::Refusal::SignedOut)) => true,
                Some(Outcome::Refused(_)) => control.form_again.load(std::sync::atomic::Ordering::SeqCst),
                _ => false,
            });
        let body = if form { "form" } else { "ready" };
        local.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await?;
        return Ok(local.shutdown().await?)
    }
    if path == SIGN_OUT_PATH {
        control.sign_outs.notify_one();
        local.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n").await?;
        return Ok(local.shutdown().await?)
    }
    if path == INDEX_PATH || !relayed(path) {
        // The patch check gets the game's own index, whatever the server or the sign-in: the game has nothing to update.
        // Nothing else of the patcher's (`ygoda/patch/…`) or any other path reaches a server (security review, #39).
        let index = if path == INDEX_PATH { control.index.lock().unwrap().clone() } else { None };
        let (status, body) = match index { Some(index) => ("200 OK", index), None => ("404 Not Found", vec![]) };
        local.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await?;
        local.write_all(&body).await?;
        return Ok(local.shutdown().await?)
    }
    let session = match wait_for_outcome(&mut active).await? {
        Outcome::Session(session) => session,
        Outcome::Refused(_) => anyhow::bail!("the sign-in was refused"),
    };
    let ticket = endpoint.ticket(&session, Role::Http).await?;
    let mut remote = endpoint.connect(&ticket, Role::Http, 0).await?;
    remote.write_all(format!("GET /{path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").as_bytes()).await?;
    if path.starts_with("web/") {
        // A page for the player's browser: whatever the server sent, it is shown sandboxed (#39).
        let mut answer = vec![];
        (&mut remote).take(WEB_PAGE_MAX).read_to_end(&mut answer).await?;
        local.write_all(&web_page(&answer)).await?;
    } else {
        tokio::io::copy(&mut remote, &mut local).await?;
    }
    local.shutdown().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A connection that fails reading or writing with `kind` (as Windows' 10053, "aborted by the software in your host
    /// machine"), and otherwise takes everything written to it.
    struct Broken { read: Option<std::io::ErrorKind>, write: Option<std::io::ErrorKind> }
    impl tokio::io::AsyncRead for Broken {
        fn poll_read(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>, _: &mut tokio::io::ReadBuf<'_>)
            -> std::task::Poll<std::io::Result<()>> {
            match self.read { Some(kind) => std::task::Poll::Ready(Err(kind.into())), None => std::task::Poll::Pending }
        }
    }
    impl tokio::io::AsyncWrite for Broken {
        fn poll_write(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>, buf: &[u8]) -> std::task::Poll<std::io::Result<usize>> {
            std::task::Poll::Ready(match self.write { Some(kind) => Err(kind.into()), None => Ok(buf.len()) })
        }
        fn poll_flush(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> { std::task::Poll::Ready(Ok(())) }
        fn poll_shutdown(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> { std::task::Poll::Ready(Ok(())) }
    }

    /// EOF after an optional interrupted read, and an optional failure while finishing the write half.
    struct Ending { interrupted: bool, shutdown_error: bool }
    impl tokio::io::AsyncRead for Ending {
        fn poll_read(mut self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>, _: &mut tokio::io::ReadBuf<'_>)
            -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(if std::mem::take(&mut self.interrupted) { Err(std::io::ErrorKind::Interrupted.into()) } else { Ok(()) })
        }
    }
    impl tokio::io::AsyncWrite for Ending {
        fn poll_write(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>, bytes: &[u8]) -> std::task::Poll<std::io::Result<usize>> {
            std::task::Poll::Ready(Ok(bytes.len()))
        }
        fn poll_flush(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> { std::task::Poll::Ready(Ok(())) }
        fn poll_shutdown(self: std::pin::Pin<&mut Self>, _: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(if self.shutdown_error { Err(std::io::ErrorKind::ConnectionAborted.into()) } else { Ok(()) })
        }
    }

    #[tokio::test]
    async fn a_relay_reports_shutdown_failures_on_either_side() {
        for (game_error, expected) in [(true, "the game's connection: "), (false, "the connection to the server: ")] {
            let mut game = Ending { interrupted: false, shutdown_error: game_error };
            let mut server = Ending { interrupted: false, shutdown_error: !game_error };
            let error = relay(&mut game, &mut server).await.unwrap_err();
            assert!(format!("{error:#}").starts_with(expected), "{error:#}");
        }
    }

    #[tokio::test]
    async fn a_relay_retries_interrupted_reads() {
        let mut game = Ending { interrupted: true, shutdown_error: false };
        let mut server = Ending { interrupted: true, shutdown_error: false };
        relay(&mut game, &mut server).await.unwrap();
    }

    #[tokio::test]
    async fn a_half_closed_relay_still_carries_the_reply() {
        let (mut game, mut game_far) = tokio::io::duplex(8);
        let (mut server, mut server_far) = tokio::io::duplex(8);
        let relayed = tokio::spawn(async move { relay(&mut game, &mut server).await });
        let exchange = async {
            game_far.write_all(b"request").await.unwrap();
            game_far.shutdown().await.unwrap();
            let mut got = Vec::new();
            server_far.read_to_end(&mut got).await.unwrap();
            assert_eq!(got, b"request");
            server_far.write_all(b"reply").await.unwrap();
            server_far.shutdown().await.unwrap();
            got.clear();
            game_far.read_to_end(&mut got).await.unwrap();
            assert_eq!(got, b"reply");
            relayed.await.unwrap().unwrap();
        };
        tokio::time::timeout(Duration::from_secs(2), exchange).await.unwrap();
    }

    #[tokio::test]
    async fn a_broken_relay_names_the_side_that_broke() {
        // #65: the helper logged "Front connection: An established connection was aborted…" (10053) and nothing said
        // whether the game's local connection or the one to the server had failed.
        let aborted = Some(std::io::ErrorKind::ConnectionAborted);
        let (mut server, _keep) = tokio::io::duplex(64);
        let error = relay(&mut Broken { read: aborted, write: None }, &mut server).await.unwrap_err();
        assert!(format!("{error:#}").starts_with("the game's connection: "), "{error:#}");
        let (mut game, mut game_far) = tokio::io::duplex(64);
        game_far.write_all(b"request").await.unwrap();
        let error = relay(&mut game, &mut Broken { read: None, write: aborted }).await.unwrap_err();
        assert!(format!("{error:#}").starts_with("the connection to the server: "), "{error:#}");
    }

    #[tokio::test]
    async fn a_relay_carries_both_ways_until_both_sides_finish() {
        let (mut game, mut game_far) = tokio::io::duplex(64);
        let (mut server, mut server_far) = tokio::io::duplex(64);
        let relayed = tokio::spawn(async move { relay(&mut game, &mut server).await });
        game_far.write_all(b"request").await.unwrap();
        let mut got = [0; 7];
        server_far.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"request");
        server_far.write_all(b"reply").await.unwrap();
        let mut got = [0; 5];
        game_far.read_exact(&mut got).await.unwrap();
        assert_eq!(&got, b"reply");
        drop(game_far);
        drop(server_far);
        tokio::time::timeout(Duration::from_secs(1), relayed).await.unwrap().unwrap().unwrap();
    }
    #[tokio::test]
    async fn desktop_control_tells_a_stop_from_the_desktop_going_away() {
        let task = tokio::spawn(wait_for_desktop_stop(std::io::Cursor::new(Vec::<u8>::new())));
        assert_eq!(tokio::time::timeout(Duration::from_secs(1), task).await.unwrap().unwrap(), DesktopStop::Gone);
        let task = tokio::spawn(wait_for_desktop_stop(std::io::Cursor::new(b"ignored\nstop\n")));
        assert_eq!(tokio::time::timeout(Duration::from_secs(1), task).await.unwrap().unwrap(), DesktopStop::Asked);
    }
    use tokio::io::AsyncWriteExt;

    #[test]
    fn the_helper_signs_out_only_a_session_it_signed_in_itself() {
        // A key profile signs in for this game alone; a password account's session is the launcher's sign-in, which
        // its next Play reuses (the second Play after closing the game found it signed out: the game said "You will
        // need an active Internet connection", Windows 11 VM, 2026-11-16). Sign out in the launcher ends that one.
        assert!(Credential::Key("k".into()).owns_session());
        assert!(!Credential::Session([7; 32]).owns_session());
    }

    #[test]
    fn a_servers_web_page_reaches_the_browser_sandboxed_with_its_headers_dropped() {
        // Security review (#39): a hostile server's page can't run script or set cookies on the proxy's origin.
        let answer = b"HTTP/1.1 200 OK\r\nContent-Type: text/javascript\r\nSet-Cookie: a=b\r\nContent-Length: 30\r\n\r\n<script>alert(1)</script><p>x</p>";
        let page = String::from_utf8(web_page(answer)).unwrap();
        let (head, body) = page.split_once("\r\n\r\n").unwrap();
        assert!(head.starts_with("HTTP/1.1 200 OK\r\n"), "{head}");
        assert!(head.contains("Content-Type: text/html; charset=utf-8") && !head.contains("javascript") && !head.contains("Set-Cookie"), "{head}");
        assert!(head.contains("Content-Security-Policy: sandbox; default-src 'none'"), "{head}");
        assert!(head.contains(&format!("Content-Length: {}", body.len())), "{head}");
        assert_eq!(body, "<script>alert(1)</script><p>x</p>");
        assert!(String::from_utf8(web_page(b"garbage")).unwrap().starts_with("HTTP/1.1 502 Error"), "no status line: a bad gateway");
    }

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
