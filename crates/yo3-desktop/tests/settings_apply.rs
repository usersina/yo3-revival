//! Settings (#31) apply without a restart: the commands the launcher builds for the next Play or server start read
//! them from the settings as they are then.
use std::ffi::OsStr;
use std::path::Path;
use yo3_desktop::{Backend, LauncherDb, Settings, options::Change};

fn root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("yo3-settings-apply-{}-{}", std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()))
}

fn env<'a>(cmd: &'a std::process::Command, key: &str) -> Option<&'a OsStr> {
    cmd.get_envs().find(|(k, _)| *k == key).and_then(|(_, v)| v)
}

fn args(cmd: &std::process::Command) -> Vec<String> { cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect() }

#[test]
fn the_next_play_takes_the_screen_size_and_wine_program_saved_since_the_launcher_opened() {
    let root = root();
    let backend = Backend::new(root.clone()).unwrap();
    let db = LauncherDb::open(&root).unwrap();
    let before = db.settings().unwrap();
    let cmd = backend.launch_command(&before, Path::new("p.json"), Path::new("ready")).unwrap();
    assert!(!args(&cmd).contains(&"--resolution".to_string()) && env(&cmd, "YO3_WINE").is_none());
    db.change(&Change::Resolution("1600x1200".into())).unwrap();
    let wine = root.join("wine");
    std::fs::write(&wine, b"").unwrap();
    let after = db.change(&Change::WineBinary(wine.to_string_lossy().into())).unwrap();
    let cmd = backend.launch_command(&after, Path::new("p.json"), Path::new("ready")).unwrap();
    let a = args(&cmd);
    assert_eq!(a[a.iter().position(|x| x == "--resolution").unwrap() + 1], "1600x1200");
    assert_eq!(env(&cmd, "YO3_WINE"), Some(wine.as_os_str()));
    // What else runs inside the game (#68): the report goes beside the ready file; Continue anyway is for one start.
    assert_eq!(a[a.iter().position(|x| x == "--survey-file").unwrap() + 1], "ready.survey.json");
    assert!(!a.contains(&"--accept-foreign".to_string()));
    backend.accept_foreign_once();
    assert!(args(&backend.launch_command(&after, Path::new("p.json"), Path::new("ready")).unwrap()).contains(&"--accept-foreign".to_string()));
    drop(backend);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn detailed_logs_reach_the_next_server_start() {
    let root = root();
    let backend = Backend::new(root.clone()).unwrap();
    let loopback = "127.0.0.1".parse().unwrap();
    let normal = Settings::default();
    assert!(env(&backend.server_command(&normal, loopback).unwrap(), "RUST_LOG").is_none());
    let mut detailed = normal.clone();
    Change::DetailedLogs(true).apply(&mut detailed).unwrap();
    assert_eq!(env(&backend.server_command(&detailed, loopback).unwrap(), "RUST_LOG"), Some(OsStr::new("debug")));
    drop(backend);
    std::fs::remove_dir_all(root).unwrap();
}
