//! Direct start (#33): the game opened without the launcher. The helper signs in for the mode and server last played
//! from the launcher: Singleplayer starts your own server when it isn't running, Multiplayer resumes the device's saved
//! sign-in. When that fails, the game says so itself: the front's reply to its login (`0x1F47`) is one of the refusals
//! LoginWindow `0x469C00` turns into its error dialog `0x46D050` (docs/protocol.md, "Login refusals").
use std::collections::BTreeMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use yo3_tunnel::{Endpoint, Token};
use crate::Credential;

/// What the game is told when the helper could not sign in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// "Failed to connect to server." (`0x21A0`, kind 1): the server is down, in maintenance or full, or your own
    /// server would not start. The game's "Undergoing Maintenance" (10608) is never referenced by the client.
    FailedToConnect,
    /// The same dialog, plus one line: the server needs a newer launcher.
    NeedsUpdate,
    /// No saved sign-in for this server: none kept, or it was revoked or expired (#34 shows the login form instead).
    SignedOut,
    /// "Incorrect User ID or Password" (`0x219B`, kind 4).
    WrongPassword,
    /// "This User ID is currently restricted." (`0x219E`, kind 6): a ban or a restriction.
    Restricted,
}

/// The one line that goes with [`Refusal::NeedsUpdate`].
pub const UPDATE_NOTICE: &str = "This server needs a newer YO3 Revival. Open YO3 Revival to update it, then start the game again.";

impl Refusal {
    /// The front's reply to the game's `0x1F47`.
    pub fn reply(self) -> u32 {
        match self {
            Refusal::WrongPassword => 0x219B,
            Refusal::Restricted => 0x219E,
            Refusal::FailedToConnect | Refusal::NeedsUpdate | Refusal::SignedOut => 0x21A0,
        }
    }
    /// The line shown beside the game's dialog, if any.
    pub fn notice(self) -> Option<&'static str> { (self == Refusal::NeedsUpdate).then_some(UPDATE_NOTICE) }
    /// From an account request's error code (yo3-server `account_ops.rs`).
    pub fn from_account_error(code: &str) -> Self {
        match code {
            "invalid_credentials" => Refusal::WrongPassword,
            "banned" | "restricted" => Refusal::Restricted,
            "update_required" => Refusal::NeedsUpdate,
            "signed_out" => Refusal::SignedOut,
            _ => Refusal::FailedToConnect,
        }
    }
}

/// A refusal and why, for the helper's log and the launcher's next start.
#[derive(Debug)]
pub struct Failure { pub refusal: Refusal, pub reason: String }

impl Failure {
    pub fn new(refusal: Refusal, reason: impl Into<String>) -> Self { Self { refusal, reason: reason.into() } }
}

/// How to start your own server for Singleplayer (written by the launcher; the same command its Play uses).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct ServerStart {
    pub program: String,
    #[serde(default)] pub args: Vec<String>,
    #[serde(default)] pub env: BTreeMap<String, String>,
    /// A file this helper touches every few seconds while it lives, for a server that can't see its standard input:
    /// Wine starts a Linux program detached, with none (the launcher wraps the server in a loop that watches it).
    #[serde(default)] pub heartbeat: Option<String>,
}

/// How often the heartbeat is touched; the launcher's wrapper allows a few times this.
pub const HEARTBEAT: Duration = Duration::from_secs(5);

/// The Linux wrapper (`sh -c WRAPPER sh HEARTBEAT PROGRAM ARGS...`): the server's standard input stays open while the
/// heartbeat file is younger than 12 s, so `--owned` stops it 30 s after the helper (the game) has gone.
pub const WRAPPER: &str = r#"f=$1; shift; while [ -f "$f" ] && [ $(( $(date +%s) - $(stat -c %Y "$f") )) -lt 12 ]; do sleep 5; echo; done | exec "$@""#;

/// How long your own server keeps running after the game exits (it stops once nobody plays for this long).
pub const SERVER_IDLE: Duration = Duration::from_secs(30);
/// How long a server started here may take to answer.
pub const SERVER_START: Duration = Duration::from_secs(45);

impl ServerStart {
    /// The command, owned by this helper: the server stops by itself `SERVER_IDLE` after this helper (the game) has gone.
    pub fn command(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.program);
        cmd.args(&self.args).args(["--owned", "--owned-idle", &SERVER_IDLE.as_secs().to_string()]).envs(&self.env)
            .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        #[cfg(windows)] cmd.creation_flags(0x08000000);
        cmd
    }
}

/// Your own server, started here: it lives while this is held (its standard input, or the heartbeat), and a little longer.
pub struct Started(#[allow(dead_code, reason = "held for its standard input")] tokio::process::Child, Option<tokio::task::JoinHandle<()>>);

impl Drop for Started { fn drop(&mut self) { if let Some(beat) = &self.1 { beat.abort() } } }

/// Starts the server when nothing answers at `endpoint` yet (a server the launcher started is reused; two starts of one
/// data folder can't both run: the server holds its data lock), and waits until it answers.
pub async fn ensure_server(endpoint: &Endpoint, start: &ServerStart, timeout: Duration) -> anyhow::Result<Option<Started>> {
    if endpoint.info().await.is_ok() { return Ok(None) }
    let beat = match &start.heartbeat {
        Some(file) => {
            std::fs::write(file, b"")?;
            let file = file.clone();
            Some(tokio::spawn(async move { loop { tokio::time::sleep(HEARTBEAT).await; let _ = std::fs::write(&file, b""); } }))
        }
        None => None,
    };
    let mut child = start.command().spawn().map_err(|e| anyhow::anyhow!("your server could not start ({e})"))?;
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if endpoint.info().await.is_ok() { return Ok(Some(Started(child, beat))) }
        if let Some(status) = child.try_wait()? {
            // Another start may have won the data lock meanwhile; then that one answers.
            if endpoint.info().await.is_ok() { return Ok(None) }
            anyhow::bail!("your server stopped during startup ({status}); its log is in the launcher's logs folder")
        }
        anyhow::ensure!(tokio::time::Instant::now() < deadline, "your server did not answer within {} seconds", timeout.as_secs());
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Reads a saved device token (the system's credential vault).
pub type ReadVault<'a> = &'a (dyn Fn(&str) -> Option<String> + Sync);

/// Signs in for the game: the server must be compatible, then the account key, the launcher's session or the saved
/// device token gives a session.
pub async fn sign_in(endpoint: &Endpoint, credential: &Credential, vault: ReadVault<'_>) -> Result<Token, Failure> {
    let info = endpoint.info().await.map_err(|e| Failure::new(Refusal::FailedToConnect, format!("the server can't be reached: {e:#}")))?;
    info.ensure_compatible(yo3_tunnel::PROTOCOL_VERSION, yo3_tunnel::MOD_ABI).map_err(|e| Failure::new(Refusal::NeedsUpdate, format!("{e:#}")))?;
    match credential {
        Credential::Key(key) => endpoint.login(key).await.map_err(|e| Failure::new(Refusal::FailedToConnect, format!("sign-in failed: {e:#}"))),
        Credential::Session(session) => Ok(*session),
        Credential::Device(account) => {
            let token = vault(account).ok_or_else(|| Failure::new(Refusal::SignedOut, "no saved sign-in for this server"))?;
            let reply = endpoint.account(&serde_json::json!({ "op": "resume", "device_token": token,
                "launcher": yo3_tunnel::BUILD_VERSION.trim_start_matches('v') })).await
                .map_err(|e| Failure::new(Refusal::FailedToConnect, format!("the server can't be reached: {e:#}")))?;
            if reply["ok"] != true {
                let code = reply["error"].as_str().unwrap_or("unknown");
                return Err(Failure::new(Refusal::from_account_error(code), format!("the server refused the saved sign-in ({code})")))
            }
            reply["session"].as_str().and_then(unhex).and_then(|b| b.try_into().ok())
                .ok_or_else(|| Failure::new(Refusal::FailedToConnect, "the server's reply has no session"))
        }
    }
}

/// What the player typed in the game's own login form (#34): its User ID and Password boxes, and Save Password.
#[derive(Deserialize, Serialize)]
pub struct Typed { pub user: String, pub password: String, #[serde(default)] pub stay: bool }

impl std::fmt::Debug for Typed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "Typed {{ user: {:?}, stay: {} }}", self.user, self.stay) }
}

/// Signs in with what was typed: a session, and with Save Password the device token kept under `vault_account` ("Stay
/// signed in": the next direct start skips the form).
pub async fn sign_in_typed(endpoint: &Endpoint, typed: &Typed, vault_account: Option<&str>, vault: &dyn crate::vault::Vault) -> Result<Token, Failure> {
    let label = format!("{} PC (the game's login)", if cfg!(windows) { "Windows" } else { "Linux" });
    let reply = endpoint.account(&serde_json::json!({ "op": "login", "user": typed.user, "password": typed.password,
        "stay": typed.stay && vault_account.is_some(), "device": label, "launcher": yo3_tunnel::BUILD_VERSION.trim_start_matches('v') })).await
        .map_err(|e| Failure::new(Refusal::FailedToConnect, format!("the server can't be reached: {e:#}")))?;
    if reply["ok"] != true {
        let code = reply["error"].as_str().unwrap_or("unknown");
        return Err(Failure::new(Refusal::from_account_error(code), format!("the server refused the sign-in ({code})")))
    }
    let session = reply["session"].as_str().and_then(unhex).and_then(|b| b.try_into().ok())
        .ok_or_else(|| Failure::new(Refusal::FailedToConnect, "the server's reply has no session"))?;
    if let (Some(account), Some(token)) = (vault_account, reply["device_token"].as_str())
        && let Err(error) = vault.set(account, token) { yo3_tunnel::diagnostics::event(&format!("Save Password: the credential store refused it ({error:#})")); }
    Ok(session)
}

/// A sign-in from the game's own login form (#34), as the helper's game loop gets it: the game's next login waits for
/// this. Signed in, it replaces `session` (ending the old one when `owns` it); refused, the form shows again.
pub async fn form_sign_in(endpoint: &Endpoint, proxy: &crate::Proxy, login: &Typed, vault_account: Option<&str>, vault: &dyn crate::vault::Vault,
    session: &mut Option<Token>, owns: bool) -> Result<(), Failure> {
    match sign_in_typed(endpoint, login, vault_account, vault).await {
        Ok(signed) => {
            if let Some(old) = session.replace(signed) && owns { let _ = endpoint.logout(&old).await; }
            proxy.activate_typed(signed);
            Ok(())
        }
        Err(failure) => { proxy.refuse_typed(failure.refusal); Err(failure) }
    }
}

/// Delete Login Data (tools/mod/signout.h): ends the device's saved sign-in on the server (best effort) and forgets it
/// here. Nothing to do for a key (Singleplayer, Scripts profiles).
pub async fn sign_out(endpoint: &Endpoint, vault_account: Option<&str>, vault: &dyn crate::vault::Vault) -> anyhow::Result<bool> {
    let Some(account) = vault_account else { return Ok(false) };
    if let Some(token) = vault.get(account)? {
        let reply = endpoint.account(&serde_json::json!({ "op": "sign_out", "device_token": token, "everywhere": false,
            "launcher": yo3_tunnel::BUILD_VERSION.trim_start_matches('v') })).await;
        if let Err(error) = &reply { yo3_tunnel::diagnostics::event(&format!("Delete Login Data: the server could not be told ({error:#})")); }
    }
    vault.delete(account)?;
    Ok(true)
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    text.len().is_multiple_of(2).then(|| (0..text.len() / 2).map(|i| u8::from_str_radix(text.get(i * 2..i * 2 + 2)?, 16).ok()).collect::<Option<Vec<u8>>>()).flatten()
}

/// Shows `line` beside the game's own dialog (a refusal's notice), without blocking the game.
pub fn notify(line: &str) {
    yo3_tunnel::diagnostics::event(&format!("Told the player: {line}"));
    #[cfg(windows)] {
        let text: Vec<u16> = line.encode_utf16().chain([0]).collect();
        let title: Vec<u16> = "YO3 Revival".encode_utf16().chain([0]).collect();
        std::thread::spawn(move || {
            use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST};
            unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION | MB_TOPMOST | MB_SETFOREGROUND) };
        });
    }
    #[cfg(not(windows))] eprintln!("{line}");
}

/// The last direct start's problem, kept beside the profile for the launcher to show next time.
pub fn record_problem(profile: &std::path::Path, failure: Option<&Failure>) {
    let file = profile.with_file_name("problem-direct.json");
    match failure {
        None => { let _ = std::fs::remove_file(file); }
        Some(failure) => {
            let body = serde_json::json!({ "refusal": format!("{:?}", failure.refusal), "reason": failure.reason,
                "at": yo3_tunnel::diagnostics::now() }).to_string();
            let _ = std::fs::write(file, body);
        }
    }
}
