//! Several games from one launcher (#64). There is still one launcher per user; it starts the main game (Play) and, with
//! **Play as…**, more games signed in to *other* accounts. Each extra game has:
//!
//! - its own local ports and server connection: it is a game helper of its own (the helper picks free ports);
//! - its own game data folder: the helper hands the connection DLL a data id derived from the server and the account
//!   (`yo3_launcher::game_data_id`), so each account's options, decks and duel logs stay apart;
//! - on Linux, its own Wine prefix (Wine allows one X display per prefix), a cheap copy of the main one made when it
//!   starts and removed when the game closes.
//!
//! The same account never plays twice (the server ends an account's older session when it signs in again), Singleplayer
//! stays at one game, and an update waits until every game is closed (the window asks `Backend::games`).
use std::path::{Path, PathBuf};
use std::process::Child;
#[cfg(not(windows))]
use anyhow::Context;
use anyhow::ensure;
use crate::{Backend, Settings, multiplayer, process, private_write};

/// The id of the main game (Play); extra games count up from 1.
pub const MAIN: u64 = 0;

/// One running game, for the list on the Multiplayer tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameInfo {
    pub id: u64,
    /// The account's User ID (Singleplayer: "Singleplayer").
    pub user: String,
    /// The server's name, and its key (the first identity), to tell which server an account plays on.
    pub server: String,
    pub server_key: String,
}

impl GameInfo { pub fn is_main(&self) -> bool { self.id == MAIN } }

/// What a started extra game keeps while it runs.
pub(crate) struct Extra {
    info: GameInfo,
    child: Child,
    errors: Option<process::Errors>,
    ready: PathBuf,
    profile: PathBuf,
    /// Its own Wine prefix (Linux), removed with the game.
    prefix: Option<PathBuf>,
}

/// What an extra game's helper is given beyond a main game's.
#[derive(Clone, Debug, Default)]
pub struct ExtraRun {
    pub prefix: Option<PathBuf>,
    pub data_key: String,
    pub clean_game: bool,
}

/// Whether two User IDs are the same account (the server ignores case).
pub fn same_account(a: &str, b: &str) -> bool { a.trim().eq_ignore_ascii_case(b.trim()) }

/// The prefixes' folder under the launcher's data.
fn prefixes(root: &Path) -> PathBuf { root.join("prefixes") }

#[cfg(not(windows))]
fn clone_prefix(source: &Path, target: &Path) -> anyhow::Result<()> {
    use std::process::{Command, Stdio};
    let run = |args: &[&std::ffi::OsStr]| Command::new("cp").args(args).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success());
    // Cheap where the file system can share data between files (btrfs, XFS): nothing is copied until it changes.
    if run(&["-a".as_ref(), "--reflink=always".as_ref(), source.as_os_str(), target.as_os_str()]) { return Ok(()) }
    let _ = std::fs::remove_dir_all(target);
    // Otherwise a shallow prefix: its own registry and settings, and the same C: drive (Wine's own files and the game
    // are only read; each game's data folder is its own, see the module's notes).
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)?.flatten() {
        if entry.file_name() == "drive_c" { std::os::unix::fs::symlink(entry.path(), target.join("drive_c"))?; continue }
        ensure!(run(&["-a".as_ref(), "--reflink=auto".as_ref(), "--".as_ref(), entry.path().as_os_str(), target.as_os_str()]), "Could not copy {} for the second game", entry.file_name().to_string_lossy());
    }
    Ok(())
}

/// The main Wine prefix games use: the Settings one, else Wine's default.
#[cfg(not(windows))]
fn main_prefix(settings: &Settings) -> PathBuf {
    if settings.wine_prefix.is_empty() { PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".wine") } else { PathBuf::from(&settings.wine_prefix) }
}

/// Removes a prefix made for a game, and only one under `prefixes`.
fn remove_prefix(root: &Path, prefix: &Path) {
    if prefix.parent() == Some(prefixes(root).as_path()) { let _ = std::fs::remove_dir_all(prefix); }
}

impl Backend {
    /// Notes who the main game is for, so the list can show it.
    pub(crate) fn note_main(&self, user: &str, server: &str, server_key: &str) {
        *self.main_label.lock().unwrap() = Some(GameInfo { id: MAIN, user: user.into(), server: server.into(), server_key: server_key.into() });
    }

    /// Every game running now, the main one first. An extra game that ended is cleaned up here: its files and Wine prefix
    /// go, and a start that failed is kept for [`Backend::take_extra_failures`].
    pub fn games(&self) -> Vec<GameInfo> {
        let mut list = vec![];
        if self.game_running() && let Some(label) = self.main_label.lock().unwrap().clone() { list.push(label) }
        let mut extras = self.extras.lock().unwrap();
        let mut ended = vec![];
        let mut at = 0;
        while at < extras.len() {
            match extras[at].child.try_wait() {
                Ok(Some(status)) => { ended.push((extras.remove(at), Some(status))) }
                Err(_) => { ended.push((extras.remove(at), None)) }
                Ok(None) => { list.push(extras[at].info.clone()); at += 1 }
            }
        }
        drop(extras);
        for (mut extra, status) in ended {
            let detail = extra.errors.take().map(|e| e.finish()).unwrap_or_default();
            let held = foreign_held(&extra.ready);
            self.diagnostics.line(&format!("The game as {} on {} ended", extra.info.user, extra.info.server));
            if held {
                self.extra_failures.lock().unwrap().push(format!("The game as {} didn't start: another program's files or software are inside the game. \
                    Press PLAY on the main screen to see what it is.", extra.info.user));
            } else if status.is_some_and(|s| !s.success()) {
                self.diagnostics.error(&format!("The game as {} stopped with an error: {}", extra.info.user, detail.trim()));
                self.extra_failures.lock().unwrap().push(format!("The game as {} stopped unexpectedly. Try Play as again; if it repeats, export logs for help.", extra.info.user));
            }
            for file in [&extra.ready, &foreign_survey(&extra.ready), &extra.profile] { let _ = std::fs::remove_file(file); }
            if let Some(prefix) = &extra.prefix { remove_prefix(&self.root, prefix) }
        }
        list
    }

    /// Why extra games didn't start or stopped, once each, as sentences.
    pub fn take_extra_failures(&self) -> Vec<String> { std::mem::take(&mut *self.extra_failures.lock().unwrap()) }

    /// Whether `user` already plays on `server_key` in a running game.
    pub fn account_playing(&self, server_key: &str, user: &str) -> bool {
        self.games().iter().any(|g| g.server_key == server_key && same_account(&g.user, user))
    }

    /// Whether launcher-started extra games are running (a Windows game list can't tell them from a game opened outside).
    pub fn has_extra_games(&self) -> bool { !self.extras.lock().unwrap().is_empty() }

    /// Starts another game on `server`, signed in as `user` with `session` (Play as…). Refused for an account that
    /// already plays on that server.
    pub fn play_extra(&self, settings: &Settings, server: &multiplayer::Server, user: &str, session: &yo3_tunnel::Token, clean_game: bool) -> anyhow::Result<()> {
        ensure!(!self.account_playing(server.key(), user), "{user} is already playing on {} in another game. Close that game first, or choose another account.", server.name);
        self.check_game(settings)?;
        ensure!(self.runtime.launcher.is_file() && self.runtime.dll.is_file() && self.runtime.injector.is_file(), "Connection files are missing. Extract the complete application bundle.");
        let id = self.next_game.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let tag = format!("{}-{id}", std::process::id());
        let prefix = self.extra_prefix(settings, &tag)?;
        let started = (|| -> anyhow::Result<Extra> {
            let profile = self.root.join("profiles").join(format!("multiplayer-{tag}.json"));
            let session: String = session.iter().map(|b| format!("{b:02x}")).collect();
            // No vault entry: a game of Play as… signs in once, and the game's Delete Login Data has nothing of it to forget.
            private_write(&profile, serde_json::json!({ "server": server.address, "server_name": server.server_name, "identity": server.key(), "session": session }).to_string().as_bytes(), true)?;
            yo3_launcher::Profile::load(&profile)?;
            let ready = self.root.join("profiles").join(format!("game-ready-{tag}.json"));
            let run = ExtraRun { prefix: prefix.clone(), data_key: user.trim().to_ascii_lowercase(), clean_game };
            let mut cmd = self.launch_command_with(settings, &profile, &ready, Some(&run))?;
            self.add_client_data(&mut cmd, settings);
            let _ = std::fs::remove_file(foreign_survey(&ready));
            let (child, errors) = process::spawn(&mut cmd, &self.diagnostics, "Game helper").inspect_err(|_| { let _ = std::fs::remove_file(&profile); })?;
            Ok(Extra { info: GameInfo { id, user: user.into(), server: server.name.clone(), server_key: server.key().into() }, child, errors: Some(errors), ready, profile, prefix: prefix.clone() })
        })();
        match started {
            Ok(extra) => { self.diagnostics.line(&format!("Started game {id} as {user} on {}", extra.info.server)); self.extras.lock().unwrap().push(extra); Ok(()) }
            Err(error) => { if let Some(prefix) = &prefix { remove_prefix(&self.root, prefix) } Err(error) }
        }
    }

    /// The Wine prefix an extra game runs in (Linux: a cheap copy of the main one), none on Windows.
    #[cfg(not(windows))]
    fn extra_prefix(&self, settings: &Settings, tag: &str) -> anyhow::Result<Option<PathBuf>> {
        let source = main_prefix(settings);
        ensure!(source.is_dir(), "Wine's prefix for the game isn't there ({}). Check Settings, Game.", source.display());
        std::fs::create_dir_all(prefixes(&self.root))?;
        let target = prefixes(&self.root).join(tag);
        clone_prefix(&source, &target).context("Could not make a second Wine prefix for the other game")?;
        Ok(Some(target))
    }
    #[cfg(windows)]
    fn extra_prefix(&self, _settings: &Settings, _tag: &str) -> anyhow::Result<Option<PathBuf>> { Ok(None) }

    /// Asks game `id` to close (its helper closes the game); the main game is [`MAIN`].
    pub fn close_game(&self, id: u64) -> anyhow::Result<()> {
        use std::io::Write;
        if id == MAIN { return self.stop_game() }
        let mut extras = self.extras.lock().unwrap();
        if let Some(extra) = extras.iter_mut().find(|e| e.info.id == id)
            && let Some(input) = &mut extra.child.stdin
            && let Err(error) = input.write_all(b"stop\n")
            && error.kind() != std::io::ErrorKind::BrokenPipe { return Err(error.into()) }
        Ok(())
    }

    /// Closes game `id` by force (it ignored the ask): its helper, and its prefix's game processes.
    pub fn force_close_game(&self, settings: &Settings, id: u64) -> anyhow::Result<()> {
        if id == MAIN { return self.force_stop_game(settings) }
        if let Some(extra) = self.extras.lock().unwrap().iter_mut().find(|e| e.info.id == id) {
            let _ = extra.child.kill(); let _ = extra.child.wait();
            #[cfg(not(windows))]
            if let Some(prefix) = &extra.prefix { for pid in crate::singleplayer::pids_in_prefix(prefix) { crate::singleplayer::kill_pid(pid) } }
        }
        self.games();
        Ok(())
    }

    /// Closes every game the launcher started (the window is closing): the extra ones are asked, given a few seconds,
    /// then forced; the main one is asked as it always was.
    pub fn close_all_games(&self, settings: &Settings) -> anyhow::Result<()> {
        let ids: Vec<u64> = self.games().iter().map(|g| g.id).filter(|&id| id != MAIN).collect();
        for &id in &ids { let _ = self.close_game(id); }
        let until = std::time::Instant::now() + std::time::Duration::from_secs(6);
        while !ids.is_empty() && std::time::Instant::now() < until && self.games().iter().any(|g| !g.is_main()) { std::thread::sleep(std::time::Duration::from_millis(150)) }
        for g in self.games().iter().filter(|g| !g.is_main()) { let _ = self.force_close_game(settings, g.id); }
        self.stop_game()
    }

    /// At the launcher's start: prefixes left by games that no longer run (a crash) are removed.
    pub(crate) fn clean_stale_prefixes(root: &Path) {
        #[cfg(not(windows))]
        for entry in std::fs::read_dir(prefixes(root)).into_iter().flatten().flatten() {
            if crate::singleplayer::pids_in_prefix(&entry.path()).is_empty() { remove_prefix(root, &entry.path()) }
        }
        #[cfg(windows)] let _ = root;
    }
}

fn foreign_survey(ready: &Path) -> PathBuf { crate::foreign::survey_file(ready) }

/// Whether the helper's report for this start says it held the game back for what runs inside it (#68, #69).
fn foreign_held(ready: &Path) -> bool { crate::foreign::Held::read(&foreign_survey(ready)).is_some() }

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yo3-games-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_same_account_is_the_same_whatever_its_case() {
        assert!(same_account("Alice", " alice "));
        assert!(!same_account("Alice", "Alicia"));
    }

    fn server(name: &str) -> multiplayer::Server {
        multiplayer::Server { name: name.into(), address: "play.example.com".into(), server_name: "play.example.com".into(), identities: vec!["abcdef0123456789".into()],
            official: false, description: String::new(), saved_user: None, last_used: 0 }
    }

    /// A backend with one extra game that is only a sleeping process, with its files and prefix in place.
    #[cfg(unix)]
    fn with_sleeping_game(name: &str, user: &str) -> (std::sync::Arc<Backend>, PathBuf, PathBuf) {
        let root = scratch(name);
        let backend = Backend::new(root.clone()).unwrap();
        let prefix = prefixes(&root).join("9-1");
        std::fs::create_dir_all(&prefix).unwrap();
        let profile = root.join("profiles").join("multiplayer-9-1.json");
        std::fs::write(&profile, b"{}").unwrap();
        let child = std::process::Command::new("sleep").arg("60").stdin(std::process::Stdio::piped()).spawn().unwrap();
        let info = GameInfo { id: 1, user: user.into(), server: "Friends".into(), server_key: server("x").key().into() };
        backend.extras.lock().unwrap().push(Extra { info, child, errors: None, ready: root.join("profiles/game-ready-9-1.json"), profile: profile.clone(), prefix: Some(prefix.clone()) });
        (backend, prefix, profile)
    }

    #[cfg(unix)]
    #[test]
    fn running_games_are_listed_with_their_account_and_server_and_the_same_account_is_refused() {
        let (backend, _, _) = with_sleeping_game("same", "Alice");
        let games = backend.games();
        assert_eq!(games.iter().map(|g| (g.user.as_str(), g.server.as_str(), g.is_main())).collect::<Vec<_>>(), [("Alice", "Friends", false)]);
        assert!(backend.account_playing(server("x").key(), " alice "), "however it is typed");
        assert!(!backend.account_playing(server("x").key(), "bob") && !backend.account_playing("another server", "alice"));
        let error = backend.play_extra(&Settings::default(), &server("Friends"), "ALICE", &[0; 32], false).unwrap_err().to_string();
        assert!(error.contains("already playing") && crate::words::technical(&error).is_none(), "a clear message: {error}");
        assert!(backend.has_extra_games());
        backend.force_close_game(&Settings::default(), 1).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn closing_an_extra_game_cleans_up_its_files_and_prefix_and_leaves_the_others() {
        let (backend, prefix, profile) = with_sleeping_game("close", "Alice");
        assert!(prefix.is_dir() && profile.is_file());
        backend.close_game(1).unwrap();
        assert_eq!(backend.games().len(), 1, "asked, but this one ignores it");
        backend.force_close_game(&Settings::default(), 1).unwrap();
        assert!(backend.games().is_empty() && !backend.has_extra_games());
        assert!(!prefix.exists() && !profile.exists(), "its prefix and its profile are gone with it");
        assert!(backend.take_extra_failures().iter().all(|f| crate::words::technical(f).is_none()));
    }

    #[cfg(unix)]
    #[test]
    fn closing_the_launcher_closes_every_extra_game() {
        let (backend, prefix, _) = with_sleeping_game("all", "Alice");
        backend.close_all_games(&Settings::default()).unwrap();
        assert!(backend.games().is_empty() && !prefix.exists());
    }

    #[test]
    fn an_extra_game_gets_its_own_prefix_and_data_key_and_free_ports() {
        let root = scratch("command");
        let backend = Backend::new(root.clone()).unwrap();
        let settings = Settings { wine_prefix: "/main/prefix".into(), ..Settings::default() };
        let args = |cmd: &std::process::Command| cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect::<Vec<_>>();
        let main = backend.launch_command(&settings, Path::new("p.json"), Path::new("ready")).unwrap();
        let run = ExtraRun { prefix: Some(PathBuf::from("/prefixes/1")), data_key: "alice".into(), clean_game: true };
        let extra = backend.launch_command_with(&settings, Path::new("q.json"), Path::new("ready2"), Some(&run)).unwrap();
        let (main, extra) = (args(&main), args(&extra));
        let after = |list: &[String], flag: &str| list.iter().position(|a| a == flag).map(|at| list[at + 1].clone());
        assert_eq!(after(&main, "--wine-prefix").as_deref(), Some("/main/prefix"));
        assert_eq!((after(&extra, "--wine-prefix").as_deref(), after(&extra, "--data-key").as_deref()), (Some("/prefixes/1"), Some("alice")));
        assert!(!main.contains(&"--data-key".to_string()), "the main game keeps its folder");
        assert!(extra.contains(&"--clean-game".to_string()) && !extra.contains(&"--accept-foreign".to_string()));
        for flag in ["--gate-port", "--front-port", "--duel-port", "--http-port"] { assert!(!extra.contains(&flag.to_string()), "{flag}: the helper picks free ports of its own") }
        assert_eq!(after(&extra, "--profile").as_deref(), Some("q.json"));
        drop(backend);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_stale_prefix_from_a_crash_is_cleaned_at_start() {
        let root = scratch("stale");
        std::fs::create_dir_all(prefixes(&root).join("1-1/drive_c")).unwrap();
        Backend::clean_stale_prefixes(&root);
        #[cfg(not(windows))]
        assert!(!prefixes(&root).join("1-1").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(not(windows))]
    #[test]
    fn a_second_prefix_is_a_copy_with_its_own_registry_and_goes_away_with_its_game() {
        let root = scratch("prefix");
        let main = root.join("main");
        std::fs::create_dir_all(main.join("drive_c/Program Files/Game")).unwrap();
        std::fs::create_dir_all(main.join("dosdevices")).unwrap();
        std::os::unix::fs::symlink("../drive_c", main.join("dosdevices/c:")).unwrap();
        std::fs::write(main.join("user.reg"), b"main registry").unwrap();
        std::fs::write(main.join("drive_c/Program Files/Game/yo3.exe"), b"game").unwrap();
        let target = prefixes(&root).join("1-1");
        std::fs::create_dir_all(prefixes(&root)).unwrap();
        clone_prefix(&main, &target).unwrap();
        assert_eq!(std::fs::read(target.join("user.reg")).unwrap(), b"main registry");
        assert_eq!(std::fs::read(target.join("drive_c/Program Files/Game/yo3.exe")).unwrap(), b"game", "the game is reachable from the copy");
        std::fs::write(target.join("user.reg"), b"second registry").unwrap();
        assert_eq!(std::fs::read(main.join("user.reg")).unwrap(), b"main registry", "its own registry: the main one is untouched");
        assert!(std::fs::symlink_metadata(target.join("dosdevices/c:")).unwrap().file_type().is_symlink());
        remove_prefix(&root, &target);
        assert!(!target.exists() && main.join("drive_c/Program Files/Game/yo3.exe").is_file(), "removed with its game, the main prefix and the game stay");
        // Only a prefix under the launcher's own folder is ever removed.
        remove_prefix(&root, &main);
        assert!(main.is_dir(), "never a folder outside the launcher's prefixes");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(not(windows))]
    #[test]
    fn the_shallow_copy_is_used_where_the_file_system_cannot_share_data_and_still_runs_the_same_game() {
        // `cp --reflink=always` fails on most temp file systems: this is the fallback's path whichever the system here.
        let root = scratch("shallow");
        let main = root.join("main");
        std::fs::create_dir_all(main.join("drive_c")).unwrap();
        std::fs::write(main.join("system.reg"), b"s").unwrap();
        std::fs::write(main.join("drive_c/file"), b"c").unwrap();
        let target = root.join("copy");
        clone_prefix(&main, &target).unwrap();
        assert!(target.join("system.reg").is_file() && target.join("drive_c/file").is_file());
        std::fs::remove_dir_all(root).unwrap();
    }
}
