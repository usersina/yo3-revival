//! A desktop keeps the control pipe open while playing. It must not prevent exit after the game closes itself.
#![cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;
mod common;

#[tokio::test]
async fn preparing_local_ports_does_not_contact_the_server_or_authenticate() {
    let dir = std::env::temp_dir().join(format!("yo3-prepare-proxy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tls = yo3_server::auth::initialize(&dir, vec!["localhost".into()]).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = yo3_tunnel::Endpoint::new(listener.local_addr().unwrap().to_string(), "localhost".into(), &tls.certificate).unwrap();
    let proxy = yo3_launcher::Proxy::prepare(endpoint, yo3_launcher::Ports::default()).await.unwrap();
    assert!(proxy.ports.front > 0 && proxy.ports.gate > 0);
    assert!(tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept()).await.is_err());
    drop(proxy); std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn missing_mod_is_reported_before_any_authentication_attempt() {
    let dir = std::env::temp_dir().join(format!("yo3-missing-mod-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
    for name in ["yo3.exe", "injector.exe"] { std::fs::write(dir.join(name), common::pe_fixture(2)).unwrap(); }
    let tls = yo3_server::auth::initialize(&dir.join("identity"), vec!["localhost".into()]).unwrap();
    let profile = dir.join("client.json");
    std::fs::write(&profile, serde_json::json!({"server":"127.0.0.1:1","server_name":"localhost","certificate":tls.certificate,"key":"x".repeat(20)}).to_string()).unwrap();
    let result = tokio::process::Command::new(env!("CARGO_BIN_EXE_yo3-launcher")).arg("--profile").arg(profile)
        .arg("--game-dir").arg(&dir).arg("--injector").arg(dir.join("injector.exe")).arg("--dll").arg(dir.join("missing.dll")).output().await.unwrap();
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(!result.status.success()); assert!(error.contains("missing.dll"), "{error}");
    assert!(!error.contains("authentication failed"), "filesystem preflight must precede authentication: {error}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn natural_game_exit_does_not_wait_for_the_desktop_to_close_stdin() {
    let dir = std::env::temp_dir().join(format!("yo3-desktop-control-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = yo3_server::Config::ephemeral(dir.clone());
    let certificate = cfg.tls.certificate.clone();
    let key = std::fs::read_to_string(dir.join("test-auth/client.key")).unwrap();
    let server = yo3_server::start(cfg).await.unwrap();
    let bin = dir.join("fake-bin"); std::fs::create_dir_all(&bin).unwrap();
    for (name, script) in [("winepath", "#!/bin/sh\nprintf '%s\\n' \"$2\"\n"), ("wine", "#!/bin/sh\nprintf 'YO3_MOD_READY\\r\\n'\nread -r resume\nsleep 0.2\nexit 0\n")] {
        let file = bin.join(name); std::fs::write(&file, script).unwrap(); std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    for name in ["yo3.exe", "injector.exe", "auth.dll"] { std::fs::write(dir.join(name), common::pe_fixture(2)).unwrap(); }
    let profile = dir.join("client.json");
    std::fs::write(&profile, serde_json::json!({"server":server.auth.to_string(),"server_name":"localhost","certificate":certificate,"key":key}).to_string()).unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_yo3-launcher"))
        .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()))
        .arg("--profile").arg(profile).arg("--game-dir").arg(&dir)
        .arg("--injector").arg(dir.join("injector.exe")).arg("--dll").arg(dir.join("auth.dll")).arg("--control-stdin")
        .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).kill_on_drop(true).spawn().unwrap();
    let _control = child.stdin.take().unwrap(); // Child::wait otherwise closes stdin itself.
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), child.wait()).await;
    assert!(result.is_ok(), "an open desktop control pipe must not keep the launcher alive after the game exits");
    assert!(result.unwrap().unwrap().success());
}
