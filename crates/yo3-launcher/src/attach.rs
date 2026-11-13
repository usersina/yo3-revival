//! Private, per-game readiness exchange for the automatically loaded game-local bootstrap.
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::io::{Read, Write};
use anyhow::ensure;
#[cfg(windows)] use anyhow::Context;
use subtle::ConstantTimeEq;
use crate::{Ports, Proxy, hex};
use yo3_tunnel::{Endpoint, Token};

fn create_private(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let mut options = std::fs::OpenOptions::new(); options.create_new(true).write(true);
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
    options.open(path)?.write_all(bytes)?; Ok(())
}
fn acknowledgement(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new(); std::fs::File::open(path)?.take(65).read_to_end(&mut bytes)?; Ok(bytes)
}
pub struct ReadyFiles { ready: PathBuf, ack: PathBuf, game_pid: u32, ports: Ports, cookie: String, server: String }
impl ReadyFiles {
    pub fn prepare(ready: &Path, ack: &Path, game_pid: u32, ports: Ports, cookie: &Token, server: &str) -> anyhow::Result<Self> {
        ensure!(std::fs::symlink_metadata(ack).is_err(), "A stale acknowledgement already exists; start a new game session");
        // Acquire ownership only after create_new succeeds; an existing user file is never overwritten/removed.
        let body = Self::payload(game_pid, ports, &hex(cookie), server, "prepared");
        create_private(ready, &body)?;
        Ok(Self { ready: ready.into(), ack: ack.into(), game_pid, ports, cookie: hex(cookie), server: server.into() })
    }
    fn payload(game_pid: u32, ports: Ports, cookie: &str, server: &str, state: &str) -> Vec<u8> {
        serde_json::json!({"state":state,"game_pid":game_pid,"helper_pid":std::process::id(),"abi":yo3_tunnel::MOD_ABI,
            "ports":ports,"cookie":cookie,"server":server}).to_string().into_bytes()
    }
    pub fn authenticated(&self) -> anyhow::Result<()> {
        let mut random = [0; 8]; getrandom::fill(&mut random)?;
        let temporary = self.ready.with_extension(format!("{}.next", hex(&random)));
        create_private(&temporary, &Self::payload(self.game_pid, self.ports, &self.cookie, &self.server, "authenticated"))?;
        let result = std::fs::rename(&temporary, &self.ready); if result.is_err() { let _ = std::fs::remove_file(&temporary); } result?; Ok(())
    }
}
impl Drop for ReadyFiles {
    fn drop(&mut self) {
        if let Ok(bytes) = acknowledgement(&self.ack)
            && bool::from(bytes.as_slice().ct_eq(self.cookie.as_bytes())) { let _ = std::fs::remove_file(&self.ack); }
        let owns = std::fs::read(&self.ready).ok().and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|v| v["helper_pid"] == std::process::id() && v["game_pid"] == self.game_pid && v["cookie"] == self.cookie);
        if owns { let _ = std::fs::remove_file(&self.ready); }
    }
}
pub async fn wait_for_ack(path: &Path, cookie: &Token, timeout: Duration, alive: impl Fn() -> anyhow::Result<bool>) -> anyhow::Result<()> {
    let expected = hex(cookie); let start = Instant::now();
    loop {
        ensure!(alive()?, "Game exited before connection-mod readiness");
        match acknowledgement(path) {
            Ok(bytes) if bytes.len() >= 64 => { ensure!(bytes.len() == 64 && bool::from(bytes.as_slice().ct_eq(expected.as_bytes())), "Connection-mod readiness belongs to a different/stale session"); return Ok(()) }
            Ok(_) => {}, Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}, Err(e) => return Err(e.into()),
        }
        ensure!(start.elapsed() < timeout, "Connection mod did not acknowledge readiness; restart or use the launcher");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
/// Only create an error file. Never replace an existing file supplied on a command line.
pub fn report_error(path: &Path, error: &str) {
    let message: String = error.chars().take(1000).collect();
    let _ = create_private(path, serde_json::json!({"state":"error","error":message,"helper_pid":std::process::id()}).to_string().as_bytes());
}

pub struct Attachment { pub pid: u32, pub game: PathBuf, pub ready: PathBuf, pub ack: PathBuf }
#[cfg(windows)]
pub async fn serve(request: Attachment, endpoint: Endpoint, key: &str, proxy: Proxy) -> anyhow::Result<()> {
    let owner = GameProcess::open(request.pid, &request.game)?;
    let files = ReadyFiles::prepare(&request.ready, &request.ack, request.pid, proxy.ports, &proxy.cookie, &proxy.server_id)?;
    wait_for_ack(&request.ack, &proxy.cookie, Duration::from_secs(30), || owner.alive()).await?;
    yo3_tunnel::diagnostics::event("Connection DLL acknowledged readiness; authenticating");
    let session = endpoint.login(key).await.context("Authentication failed; game login remains blocked")?;
    let result = async {
        ensure!(owner.alive()?, "Game exited before authentication completed");
        proxy.activate(session); files.authenticated()?;
        yo3_tunnel::diagnostics::event("Authenticated; direct-start game released");
        while owner.alive()? { tokio::time::sleep(Duration::from_millis(250)).await; }
        Ok(())
    }.await;
    let _ = endpoint.logout(&session).await;
    result
}
#[cfg(not(windows))]
pub async fn serve(_: Attachment, _: Endpoint, _: &str, _: Proxy) -> anyhow::Result<()> {
    anyhow::bail!("Direct game attachment uses the Windows helper, including the supplied helper under Wine")
}

#[cfg(windows)]
struct GameProcess(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl GameProcess {
    fn open(pid: u32, expected: &Path) -> anyhow::Result<Self> {
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE};
        ensure!(pid != 0 && pid != std::process::id(), "Invalid game process");
        let process = Self(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, 0, pid) });
        ensure!(!process.0.is_null(), "Game process is unavailable");
        let mut buffer = vec![0; 32768]; let mut length = buffer.len() as u32;
        ensure!(unsafe { QueryFullProcessImageNameW(process.0, 0, buffer.as_mut_ptr(), &mut length) } != 0, "Cannot verify game process executable");
        let actual = std::path::PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length as usize]));
        ensure!(actual.canonicalize()?.to_string_lossy().to_lowercase() == expected.canonicalize()?.to_string_lossy().to_lowercase(), "Attachment process does not match the configured game executable");
        Ok(process)
    }
    fn alive(&self) -> anyhow::Result<bool> {
        let status = unsafe { windows_sys::Win32::System::Threading::WaitForSingleObject(self.0, 0) };
        ensure!(status == 0 || status == 258, "Cannot check game process state"); Ok(status == 258)
    }
}
#[cfg(windows)]
impl Drop for GameProcess { fn drop(&mut self) { if !self.0.is_null() { unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0); } } } }
