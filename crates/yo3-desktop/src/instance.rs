//! One launcher per user: a second start brings the first window to the front instead of opening another window.
//!
//! The owner holds an exclusive lock on `<app data>/instance.lock` for as long as it runs (the operating system drops
//! it when the process ends, even after a crash) and listens on a loopback port. `instance.json` names the port and a
//! random token. A second start finds the lock taken, sends the token to that port and exits.
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::time::{Duration, Instant};
use anyhow::Context;

const TOKEN_LEN: usize = 32;

/// What a start found.
pub enum Start {
    /// This process is the launcher; keep the [`Owner`] alive and [`Owner::serve`] it.
    Owner(Owner),
    /// Another launcher was asked to come to the front; this process should exit.
    Forwarded,
}

/// The running launcher's claim. Dropping it releases the lock.
pub struct Owner { _lock: File, listener: TcpListener, token: String }

impl Owner {
    /// Calls `show` each time another start asks for the window. Blocks: run it on its own thread.
    pub fn serve(self, mut show: impl FnMut()) {
        for stream in self.listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut received = [0u8; TOKEN_LEN];
            if stream.read_exact(&mut received).is_ok() && received == self.token.as_bytes() { show() }
        }
    }
}

/// Claims the launcher for this user, or forwards to the one already running.
pub fn claim(root: &Path) -> anyhow::Result<Start> {
    let lock = lock_file(root)?;
    match lock.try_lock() {
        Ok(()) => Ok(Start::Owner(own(root, lock)?)),
        Err(TryLockError::WouldBlock) => {
            forward(root).context("YO3 Revival is already open, but it did not answer. Close it, then open YO3 Revival again.")?;
            Ok(Start::Forwarded)
        }
        Err(TryLockError::Error(error)) => Err(error).context("Could not check whether YO3 Revival is already open"),
    }
}

/// Waits until the launcher can be claimed: an update restart opens while the previous version is still closing.
pub fn claim_when_free(root: &Path, timeout: Duration) -> anyhow::Result<Owner> {
    let lock = lock_file(root)?;
    let until = Instant::now() + timeout;
    loop {
        match lock.try_lock() {
            Ok(()) => return own(root, lock),
            Err(TryLockError::WouldBlock) if Instant::now() < until => std::thread::sleep(Duration::from_millis(250)),
            Err(TryLockError::WouldBlock) => anyhow::bail!("The previous YO3 Revival did not close"),
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
    }
}

fn lock_file(root: &Path) -> anyhow::Result<File> {
    std::fs::create_dir_all(root)?;
    Ok(OpenOptions::new().create(true).truncate(false).read(true).write(true).open(root.join("instance.lock"))?)
}

fn own(root: &Path, lock: File) -> anyhow::Result<Owner> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let mut bytes = [0u8; TOKEN_LEN / 2];
    ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut bytes).map_err(|_| anyhow::anyhow!("No random source"))?;
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let note = serde_json::json!({ "port": listener.local_addr()?.port(), "token": token });
    crate::private_write(&root.join("instance.json"), note.to_string().as_bytes(), true)?;
    Ok(Owner { _lock: lock, listener, token })
}

fn forward(root: &Path) -> anyhow::Result<()> {
    let note: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("instance.json"))?)?;
    let port = note["port"].as_u64().and_then(|p| u16::try_from(p).ok()).context("No port")?;
    let token = note["token"].as_str().filter(|t| t.len() == TOKEN_LEN).context("No token")?;
    let mut stream = TcpStream::connect_timeout(&SocketAddr::from((Ipv4Addr::LOCALHOST, port)), Duration::from_secs(2))?;
    stream.write_all(token.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scratch() -> std::path::PathBuf {
        let at = std::env::temp_dir().join(format!("yo3-instance-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&at).unwrap();
        at
    }
    use std::sync::mpsc;

    #[test]
    fn a_second_start_brings_the_first_forward_and_exits() {
        let root = scratch();
        let Start::Owner(owner) = claim(root.as_path()).unwrap() else { panic!("the first start owns the launcher") };
        let (shown, seen) = mpsc::channel();
        std::thread::spawn(move || owner.serve(move || { let _ = shown.send(()); }));
        assert!(matches!(claim(root.as_path()).unwrap(), Start::Forwarded));
        seen.recv_timeout(Duration::from_secs(5)).expect("the owner was asked to show its window");
    }

    #[test]
    fn a_wrong_token_is_ignored() {
        let root = scratch();
        let Start::Owner(owner) = claim(root.as_path()).unwrap() else { panic!() };
        let port = owner.listener.local_addr().unwrap().port();
        let (shown, seen) = mpsc::channel();
        std::thread::spawn(move || owner.serve(move || { let _ = shown.send(()); }));
        TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap().write_all(&[b'x'; TOKEN_LEN]).unwrap();
        assert!(seen.recv_timeout(Duration::from_millis(500)).is_err());
    }

    #[test]
    fn the_claim_frees_when_the_owner_goes_and_an_update_restart_waits_for_it() {
        let root = scratch();
        let Start::Owner(owner) = claim(root.as_path()).unwrap() else { panic!() };
        assert!(claim_when_free(root.as_path(), Duration::from_millis(300)).is_err(), "still held");
        let path = root.as_path().to_owned();
        let waiter = std::thread::spawn(move || claim_when_free(&path, Duration::from_secs(5)).map(|_| ()));
        std::thread::sleep(Duration::from_millis(400));
        drop(owner);
        waiter.join().unwrap().expect("claimed once the previous launcher let go");
    }
}
