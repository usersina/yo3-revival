//! The setup wizard's folder checks (#28) against fake game folders: none, the wrong build, missing data, read-only
//! and a non-ASCII path; with the real game when it is installed.
use std::path::{Path, PathBuf};
use yo3_desktop::setup_check::{Check, State, fingerprint, game_lines, passes};
#[path = "../../yo3-launcher/tests/common/mod.rs"]
#[allow(dead_code, reason = "shared fixtures; this file uses only the game image")]
mod common;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("yo3-setup-{name}-{}-{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
fn state(lines: &[yo3_desktop::setup_check::Line], check: Check) -> Option<State> { lines.iter().find(|l| l.check == check).map(|l| l.state) }

#[test]
fn a_folder_without_yo3_exe_asks_for_the_right_folder() {
    let lines = game_lines(&scratch("empty"));
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].state, State::Fail);
    assert!(lines[0].sentence.contains("no yo3.exe"), "{}", lines[0].sentence);
    assert!(!passes(&lines));
}

#[test]
fn the_wrong_build_is_named_as_such() {
    let dir = scratch("wrong");
    std::fs::write(dir.join("yo3.exe"), b"MZ not the game").unwrap();
    let lines = game_lines(&dir);
    assert_eq!(state(&lines, Check::Game), Some(State::Fail));
    assert!(lines[0].sentence.contains("isn't the supported version"));
    assert!(lines[0].details.is_some(), "the reason stays behind Details");
}

#[test]
fn the_supported_build_without_its_data_fails_only_the_data_line() {
    let dir = scratch("nodata");
    std::fs::write(dir.join("yo3.exe"), common::game_fixture()).unwrap();
    let lines = game_lines(&dir);
    assert_eq!(state(&lines, Check::Game), Some(State::Pass));
    assert_eq!(state(&lines, Check::Data), Some(State::Fail));
    assert_eq!(state(&lines, Check::DirectStart), Some(State::Pass));
    assert!(!dir.join(".yo3-revival-write-check").exists(), "the write check cleans up after itself");
}

#[test]
fn a_non_ascii_path_is_checked_like_any_other() {
    let dir = scratch("ユーギ王 Öffnen");
    std::fs::write(dir.join("yo3.exe"), common::game_fixture()).unwrap();
    assert_eq!(state(&game_lines(&dir), Check::Game), Some(State::Pass));
}

#[cfg(unix)]
#[test]
fn a_read_only_folder_only_warns_about_direct_start() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("readonly");
    std::fs::write(dir.join("yo3.exe"), common::game_fixture()).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let probe = std::fs::write(dir.join("probe"), b"");
    let lines = game_lines(&dir);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    if probe.is_ok() { return } // running as root: permissions don't apply
    let line = lines.iter().find(|l| l.check == Check::DirectStart).unwrap();
    assert_eq!(line.state, State::Warn);
    assert!(line.sentence.contains("protected") || line.sentence.contains("read-only"), "{}", line.sentence);
}

#[test]
fn the_fingerprint_changes_when_the_game_changes() {
    let dir = scratch("fingerprint");
    std::fs::write(dir.join("yo3.exe"), common::game_fixture()).unwrap();
    let before = fingerprint(&dir).unwrap();
    assert_eq!(before, fingerprint(&dir).unwrap());
    let mut changed = common::game_fixture(); let last = changed.len() - 1; changed[last] ^= 1;
    std::fs::write(dir.join("yo3.exe"), changed).unwrap();
    assert_ne!(before, fingerprint(&dir).unwrap());
    assert!(fingerprint(Path::new("/nonexistent")).is_err());
}

/// With the real game installed (YO3_GAME_DIR or the default folder), the game and data lines pass.
#[test]
fn the_installed_game_passes_when_present() {
    let dir = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
    if !dir.join("yo3.exe").is_file() { return }
    let lines = game_lines(&dir);
    assert_eq!(state(&lines, Check::Game), Some(State::Pass), "{lines:?}");
    assert_eq!(state(&lines, Check::Data), Some(State::Pass), "{lines:?}");
}

#[cfg(not(windows))]
#[test]
fn a_missing_wine_prefix_says_so() {
    let line = yo3_desktop::setup_check::wine_line(Some(Path::new("/nonexistent/prefix")));
    assert_eq!((line.check, line.state), (Check::Wine, State::Fail));
    assert!(line.sentence.contains("prefix doesn't exist"), "{}", line.sentence);
}

/// A wrong game folder is the game's problem: YO3 Revival's own files are not blamed for it (they are checked
/// against yo3.exe, so they can only be judged once the game is found).
#[test]
fn a_wrong_folder_never_reads_as_our_files_needing_repair() {
    let backend = yo3_desktop::Backend::new(scratch("backend")).unwrap();
    let settings = yo3_desktop::Settings { game_dir: scratch("nogame").to_string_lossy().into_owned(), ..Default::default() };
    let lines = backend.setup_checklist(&settings);
    assert_eq!(state(&lines, Check::Game), Some(State::Fail));
    assert_eq!(state(&lines, Check::Ours), None, "{lines:?}");
    assert_eq!(state(&lines, Check::Runtimes), None, "nothing to check the runtimes against");
}
