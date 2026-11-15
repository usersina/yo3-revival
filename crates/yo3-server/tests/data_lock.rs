//! One data folder, one owner. Two servers on one folder (Desktop and Scripts, or one started twice on other ports)
//! each kept their own copy of the accounts and the last save won: two BP grants of 100 and 200 left 5200, not
//! 5300 (review, 2026-11-15). The server and the offline admin commands now hold an OS lock on the folder for as
//! long as they run; the OS lets it go when the process ends, however it ends.
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const HOLDER: &str = "YO3_TEST_HOLD_DATA_LOCK";

/// Run as a child (`holder`): holds the lock on the folder in YO3_TEST_HOLD_DATA_LOCK until killed.
#[test]
#[ignore = "a helper process for the tests below"]
fn holder() {
    let Some(dir) = std::env::var_os(HOLDER) else { return };
    let _lock = yo3_server::store::lock(Path::new(&dir)).expect("the holder takes the lock");
    println!("locked");
    std::thread::sleep(Duration::from_secs(120));
}

/// Another process holding the lock on `dir`, once it has it.
fn hold(dir: &Path) -> Child {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "holder", "--ignored", "--nocapture"])
        .env(HOLDER, dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while yo3_server::store::lock(dir).is_ok() {
        assert!(Instant::now() < deadline, "the holder never took the lock");
        assert!(child.try_wait().unwrap().is_none(), "the holder ended early");
        std::thread::sleep(Duration::from_millis(20));
    }
    child
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("yo3-data-lock-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The server binary with `args`, given 10 s to end.
fn server(data: &Path, args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_yo3-server"))
        .arg("--data-dir").arg(data)
        .arg("--log-dir").arg(data.join("logs"))
        .arg("--game-dir").arg(data.join("no-game"))
        .args(args)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("{args:?} did not end");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    child.wait_with_output().unwrap()
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

#[test]
fn a_second_server_on_the_same_folder_is_refused_whatever_its_ports() {
    let root = scratch("server");
    let data = root.join("data");
    yo3_server::auth::initialize(&data, vec!["localhost".into()]).unwrap();
    let certificate = std::fs::read(data.join("server.crt")).unwrap();
    let mut first = hold(&data);
    for ports in [["--auth-port", "0", "--http-port", "0"], ["--auth-port", "17999", "--http-port", "18999"]] {
        let second = server(&data, &ports);
        assert!(!second.status.success());
        assert!(text(&second).contains("in use by another"), "{}", text(&second));
    }
    assert_eq!(std::fs::read(data.join("server.crt")).unwrap(), certificate, "the refused server renewed nothing");
    assert!(!data.join("server.db").exists(), "the refused server made no player storage");
    first.kill().unwrap();
    first.wait().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn offline_admin_commands_wait_for_the_folder_and_get_it_once_its_owner_ends() {
    let root = scratch("admin");
    let data = root.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let mut owner = hold(&data);
    let refused = server(&data, &["add-account", "Yugi"]);
    assert!(!refused.status.success());
    assert!(text(&refused).contains("in use by another"), "{}", text(&refused));
    // Killed, not stopped: the OS releases the lock all the same.
    owner.kill().unwrap();
    owner.wait().unwrap();
    let made = server(&data, &["add-account", "Yugi"]);
    assert!(made.status.success(), "{}", text(&made));
    let listed = server(&data, &["accounts"]);
    assert!(text(&listed).contains("Yugi"), "{}", text(&listed));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_lock_is_the_os_hold_not_the_file() {
    // A lock file left behind by a crash does not keep the folder busy.
    let dir = scratch("stale");
    std::fs::write(dir.join("server.lock"), "pid 1\n").unwrap();
    let lock = yo3_server::store::lock(&dir).expect("a stale lock file is no owner");
    assert!(yo3_server::store::lock(&dir).is_err(), "a held lock refuses a second owner, in this process too");
    drop(lock);
    assert!(yo3_server::store::lock(&dir).is_ok(), "dropped, it is free");
    std::fs::remove_dir_all(dir).unwrap();
}
