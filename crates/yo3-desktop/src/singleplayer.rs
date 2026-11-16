//! Singleplayer's lifetime (#29): your own server lives in `<app data>/singleplayer/`, starts with Play, stops a grace
//! period after the game exits, and stops itself if the launcher goes away (`--owned`: the launcher holds the
//! server's standard input). Also: finding a game opened without the launcher, closing the game by force when it
//! ignores a stop, and restoring the server's database from one of its start-up backups.
use std::path::{Path, PathBuf};
use anyhow::Context;
use crate::{Backend, Settings};

/// How long your server waits after the game exits, so a quick relaunch reuses it.
pub const GRACE: std::time::Duration = std::time::Duration::from_secs(30);
/// How long Close game waits for the game to stop before forcing it.
pub const CLOSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// A start-up backup of the server's database, newest first in [`Backend::backups`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backup { pub dir: PathBuf, pub taken: String }

impl std::fmt::Display for Backup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.taken) }
}

impl Backend {
    /// The game's processes for this game's prefix (Linux) or on this PC (Windows), apart from the one Play started.
    pub fn game_open_elsewhere(&self, settings: &Settings) -> bool {
        !self.game_running() && !game_pids(settings).is_empty()
    }

    /// Stops the game Play started, by force: the helper, then every game process of this prefix.
    pub fn force_stop_game(&self, settings: &Settings) -> anyhow::Result<()> {
        if let Some(child) = self.game.lock().unwrap().as_mut() { let _ = child.kill(); let _ = child.wait(); }
        for pid in game_pids(settings) { kill(pid) }
        self.clear_game_ready();
        Ok(())
    }

    /// The server's start-up backups, newest first.
    pub fn backups(&self, settings: &Settings) -> Vec<Backup> {
        backups_in(&self.data_dir(settings).join("backups"))
    }

    /// Puts `backup` back as the server's database. The damaged one is kept beside it, renamed. Your server must be
    /// stopped.
    pub fn restore_backup(&self, settings: &Settings, backup: &Backup) -> anyhow::Result<()> {
        anyhow::ensure!(self.live(settings).is_none(), "Stop your server before restoring a backup.");
        let data = self.data_dir(settings);
        let source = backup.dir.join("server.db");
        anyhow::ensure!(source.is_file() && backup.dir.starts_with(data.join("backups")), "That backup is incomplete.");
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        for suffix in ["", "-wal", "-shm"] {
            let current = data.join(format!("server.db{suffix}"));
            if current.exists() { std::fs::rename(&current, data.join(format!("server.db{suffix}.damaged-{stamp}")))?; }
        }
        std::fs::copy(&source, data.join("server.db")).context("Could not copy the backup")?;
        self.diagnostics.line(&format!("Restored the server database from the backup taken {}", backup.taken));
        Ok(())
    }

    /// Settings → Singleplayer → Back up: a copy of the server's database now, beside the start-up ones. Works while
    /// the server runs (SQLite's online backup).
    pub fn back_up_now(&self, settings: &Settings) -> anyhow::Result<()> {
        let data = self.data_dir(settings);
        anyhow::ensure!(data.join("server.db").is_file(), "There is nothing to back up yet: play once first.");
        yo3_server::store::back_up(&data).context("Could not back up the server's database")?;
        self.diagnostics.line("Backed up the singleplayer database on request");
        Ok(())
    }

    /// Settings → Singleplayer → Reset: your server starts again with new data. The old data folder is kept beside it,
    /// renamed (its name returned). Your server must be stopped.
    pub fn reset_singleplayer(&self, settings: &Settings) -> anyhow::Result<PathBuf> {
        anyhow::ensure!(self.live(settings).is_none(), "Stop your server before resetting its data.");
        anyhow::ensure!(!self.game_running(), "Close the game before resetting your singleplayer data.");
        let data = self.data_dir(settings);
        anyhow::ensure!(data.is_dir(), "There is no singleplayer data to reset.");
        let kept = data.with_file_name(format!("{}-reset-{}", data.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            chrono::Local::now().format("%Y%m%d-%H%M%S")));
        std::fs::rename(&data, &kept).context("Could not move the old singleplayer data aside")?;
        self.diagnostics.line(&format!("Reset singleplayer data; the old data is in {}", kept.display()));
        Ok(kept)
    }
}

/// Whether a failed start of your server looks like a damaged database (offer a backup then).
pub fn damaged_database(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    ["database disk image is malformed", "file is not a database", "server.db", "sqlite"].iter().any(|s| lower.contains(s))
}

fn backups_in(dir: &Path) -> Vec<Backup> {
    let mut found: Vec<Backup> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path())
        .filter(|p| p.join("server.db").is_file())
        .map(|dir| {
            let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            // Folder names are the backup's local time, YYYYmmdd-HHMMSS[.nanos].
            let taken = chrono::NaiveDateTime::parse_from_str(name.get(..15).unwrap_or(&name), "%Y%m%d-%H%M%S")
                .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or(name);
            Backup { dir, taken }
        }).collect();
    found.sort_by(|a, b| b.dir.cmp(&a.dir));
    found
}

#[cfg(not(windows))]
fn game_pids(settings: &Settings) -> Vec<u32> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let ours = if settings.wine_prefix.is_empty() { home.join(".wine") } else { PathBuf::from(&settings.wine_prefix) };
    let ours = ours.canonicalize().unwrap_or(ours);
    std::fs::read_dir("/proc").into_iter().flatten().flatten().filter_map(|entry| {
        let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
        let name = std::fs::read(entry.path().join("comm")).ok()?;
        if !name.strip_suffix(b"\n").unwrap_or(&name).eq_ignore_ascii_case(b"yo3.exe") { return None }
        // Only games of this prefix: another prefix's client (another account, a test client) isn't this game.
        let environ = std::fs::read(entry.path().join("environ")).ok()?;
        let prefix = environ.split(|b| *b == 0).find_map(|v| v.strip_prefix(b"WINEPREFIX=")).map(|v| PathBuf::from(String::from_utf8_lossy(v).into_owned()))
            .unwrap_or_else(|| home.join(".wine"));
        (prefix.canonicalize().unwrap_or(prefix) == ours).then_some(pid)
    }).collect()
}

#[cfg(not(windows))]
fn kill(pid: u32) { let _ = std::process::Command::new("kill").args(["-KILL", &pid.to_string()]).status(); }

#[cfg(windows)]
fn game_pids(_settings: &Settings) -> Vec<u32> {
    use windows_sys::Win32::{Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS}};
    let mut pids = Vec::new();
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE { return pids }
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut found = unsafe { Process32FirstW(snapshot, &mut entry) };
    while found != 0 {
        let end = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
        if String::from_utf16_lossy(&entry.szExeFile[..end]).eq_ignore_ascii_case("yo3.exe") { pids.push(entry.th32ProcessID) }
        found = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    unsafe { CloseHandle(snapshot) };
    pids
}

#[cfg(windows)]
fn kill(pid: u32) {
    use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE}};
    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if !handle.is_null() { unsafe { TerminateProcess(handle, 1); CloseHandle(handle); } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backups_are_listed_newest_first_with_readable_times() {
        let dir = std::env::temp_dir().join(format!("yo3-backups-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        for name in ["20261114-090000.000000001", "20261115-101500.000000002", "incomplete"] {
            std::fs::create_dir_all(dir.join(name)).unwrap();
            if name != "incomplete" { std::fs::write(dir.join(name).join("server.db"), b"x").unwrap() }
        }
        let list = backups_in(&dir);
        assert_eq!(list.iter().map(|b| b.taken.as_str()).collect::<Vec<_>>(), ["2026-11-15 10:15:00", "2026-11-14 09:00:00"]);
    }

    #[test]
    fn a_reset_keeps_the_old_data_beside_and_a_backup_needs_a_database() {
        let root = std::env::temp_dir().join(format!("yo3-reset-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let backend = Backend::new(root.clone()).unwrap();
        let settings = Settings::default();
        assert!(backend.reset_singleplayer(&settings).is_err(), "nothing to reset yet");
        assert!(backend.back_up_now(&settings).is_err(), "nothing to back up yet");
        let data = backend.data_dir(&settings);
        yo3_server::store::initialize(&data).unwrap();
        backend.back_up_now(&settings).unwrap();
        assert_eq!(backend.backups(&settings).len(), 1);
        let kept = backend.reset_singleplayer(&settings).unwrap();
        assert!(!data.exists() && kept.join("server.db").is_file() && kept.parent() == data.parent());
        drop(backend);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_damaged_database_is_recognised() {
        assert!(damaged_database("Server startup failed: database disk image is malformed"));
        assert!(damaged_database("file is not a database"));
        assert!(!damaged_database("TCP port 7443 is already used by another program"));
    }
}
