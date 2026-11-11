//! A desktop keeps the control pipe open while playing. It must not prevent exit after the game closes itself.
#![cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Stdio;
mod common;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_can_cancel_while_waiting_for_the_mod() { cancel_initialization("wine",false).await; }

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_can_cancel_runtime_validation() { cancel_initialization("runtime",false).await; }

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn desktop_can_cancel_a_stalled_winepath() { cancel_initialization("winepath",false).await; }

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scripts_can_interrupt_a_stalled_winepath() { cancel_initialization("winepath",true).await; }

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scripts_can_interrupt_while_waiting_for_the_mod() { cancel_initialization("wine",true).await; }

async fn cancel_initialization(stalled: &str, interrupt:bool) {
    use tokio::io::AsyncWriteExt;
    let dir=std::env::temp_dir().join(format!("yo3-cancel-{stalled}-{interrupt}-{}",std::process::id())); let _=std::fs::remove_dir_all(&dir);
    let cfg=yo3_server::Config::ephemeral(dir.clone()); let certificate=cfg.tls.certificate.clone();
    let key=std::fs::read_to_string(dir.join("test-auth/client.key")).unwrap(); let server=yo3_server::start(cfg).await.unwrap();
    let bin=dir.join("fake-bin");std::fs::create_dir_all(&bin).unwrap();let marker=dir.join("waiting");
    for (name,script) in [("winepath","#!/bin/sh\nprintf '%s\\n' \"$2\"\n"),("wine","#!/bin/sh\nprintf 'YO3_MOD_READY\\n'\nread -r resume\nexit 0\n")] {
        let file=bin.join(name);std::fs::write(&file,if name==stalled {"#!/bin/sh\necho $$ > \"$YO3_TEST_WAITING\"\nexec sleep 60\n"}else{script}).unwrap();
        if name == "wine" && stalled == "wine" {
            std::fs::write(&file,"#!/bin/sh\nif [ \"$2\" = --check-runtime ]; then exit 0; fi\necho $$ > \"$YO3_TEST_WAITING\"\nexec sleep 60\n").unwrap();
        } else if name == "wine" && stalled == "runtime" {
            std::fs::write(&file,"#!/bin/sh\necho $$ > \"$YO3_TEST_WAITING\"\nexec sleep 60\n").unwrap();
        }
        std::fs::set_permissions(file,std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(dir.join("yo3.exe"),common::game_fixture()).unwrap();
    for name in ["injector.exe","auth.dll"] {std::fs::write(dir.join(name),common::pe_fixture(2)).unwrap();}
    let digest=yo3_launcher::components::hash(&common::pe_fixture(2));
    std::fs::write(dir.join("bundle.json"),serde_json::json!({"release":yo3_tunnel::BUILD_VERSION,"mod_abi":yo3_tunnel::MOD_ABI,
        "components":{"injector":{"sha256":digest},"connection_mod":{"sha256":digest}}}).to_string()).unwrap();
    let profile=dir.join("client.json");std::fs::write(&profile,serde_json::json!({"server":server.auth.to_string(),"server_name":"localhost","certificate":certificate,"key":key}).to_string()).unwrap();
    let mut command=tokio::process::Command::new(env!("CARGO_BIN_EXE_yo3-launcher"));
    command.env("PATH",format!("{}:{}",bin.display(),std::env::var("PATH").unwrap())).env("YO3_TEST_WAITING",&marker)
        .arg("--profile").arg(profile).arg("--game-dir").arg(&dir).arg("--injector").arg(dir.join("injector.exe"))
        .arg("--dll").arg(dir.join("auth.dll"));
    if !interrupt {command.arg("--control-stdin");}
    let mut child=command.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).kill_on_drop(true).spawn().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3),async {while !marker.is_file(){tokio::time::sleep(std::time::Duration::from_millis(10)).await;}}).await.unwrap();
    let mut control=child.stdin.take().unwrap();
    if interrupt {assert!(std::process::Command::new("kill").args(["-INT",&child.id().unwrap().to_string()]).status().unwrap().success());}
    else {control.write_all(b"stop\n").await.unwrap();}
    let stopped=tokio::time::timeout(std::time::Duration::from_secs(2),child.wait()).await;
    assert!(stopped.is_ok(),"Close game must cancel {stalled} before DLL readiness, with the control pipe still open");
    assert!(stopped.unwrap().unwrap().success(),"startup cancellation must run cleanup and exit successfully");std::fs::remove_dir_all(dir).unwrap();
}

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
    std::fs::write(dir.join("yo3.exe"),common::game_fixture()).unwrap();
    std::fs::write(dir.join("injector.exe"),common::pe_fixture(2)).unwrap();
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
    std::fs::write(dir.join("yo3.exe"),common::game_fixture()).unwrap();
    for name in ["injector.exe", "auth.dll"] { std::fs::write(dir.join(name), common::pe_fixture(2)).unwrap(); }
    let digest=yo3_launcher::components::hash(&common::pe_fixture(2));
    std::fs::write(dir.join("bundle.json"),serde_json::json!({"release":yo3_tunnel::BUILD_VERSION,"mod_abi":yo3_tunnel::MOD_ABI,
        "components":{"injector":{"sha256":digest},"connection_mod":{"sha256":digest}}}).to_string()).unwrap();
    let profile = dir.join("client.json");
    std::fs::write(&profile, serde_json::json!({"server":server.auth.to_string(),"server_name":"localhost","certificate":certificate,"key":key}).to_string()).unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_yo3-launcher"))
        .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()))
        .arg("--profile").arg(profile).arg("--game-dir").arg(&dir)
        .arg("--injector").arg(dir.join("injector.exe")).arg("--dll").arg(dir.join("auth.dll")).arg("--control-stdin")
        .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).kill_on_drop(true).spawn().unwrap();
    let _control = child.stdin.take().unwrap(); // Child::wait otherwise closes stdin itself.
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), child.wait_with_output()).await;
    assert!(result.is_ok(), "an open desktop control pipe must not keep the launcher alive after the game exits");
    let output=result.unwrap().unwrap(); assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runtime_failure_is_actionable_before_connecting_or_launching_the_game() { startup_failure(true).await; }

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_mod_failure_reaches_the_desktop_and_logs_without_credentials() { startup_failure(false).await; }

async fn startup_failure(runtime: bool) {
    let dir = std::env::temp_dir().join(format!("yo3-native-failure-{runtime}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = yo3_server::Config::ephemeral(dir.clone()); let certificate = cfg.tls.certificate.clone();
    let key = std::fs::read_to_string(dir.join("test-auth/client.key")).unwrap();
    let server = yo3_server::start(cfg).await.unwrap();
    let bin = dir.join("fake-bin"); std::fs::create_dir_all(&bin).unwrap();
    let script = if runtime {
        "#!/bin/sh\nif [ \"$2\" = --check-runtime ]; then echo 'stage=runtime-dependency windows_error=126: d3dx9_40.dll missing; install DirectX End-User Runtimes (June 2010)' >&2; exit 1; fi\ntouch \"$YO3_TEST_LAUNCHED\"\nexit 1\n"
    } else {
        "#!/bin/sh\nif [ \"$2\" = --check-runtime ]; then exit 0; fi\npython3 - <<'PY'\nimport os,sys\nstage='stage=mod-load windows_error=1114: Connection DLL initialization failed; validate game and export logs'\npadding=8192-12-1-1-len(stage)-1\nsys.stderr.write('debug '+os.environ['YO3_TEST_SECRET']+'\\n'+'x'*padding+'\\n'+stage+'\\n')\nPY\nexit 1\n"
    };
    for (name, script) in [("winepath", "#!/bin/sh\nprintf '%s\\n' \"$2\"\n"), ("wine", script)] {
        let file = bin.join(name); std::fs::write(&file, script).unwrap();
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(dir.join("yo3.exe"), common::game_fixture()).unwrap();
    for name in ["injector.exe", "auth.dll"] { std::fs::write(dir.join(name), common::pe_fixture(2)).unwrap(); }
    let digest = yo3_launcher::components::hash(&common::pe_fixture(2));
    std::fs::write(dir.join("bundle.json"), serde_json::json!({"release":yo3_tunnel::BUILD_VERSION,"mod_abi":2,
        "components":{"injector":{"sha256":digest},"connection_mod":{"sha256":digest}}}).to_string()).unwrap();
    let profile = dir.join("client.json");
    std::fs::write(&profile, serde_json::json!({"server":if runtime { "127.0.0.1:1".to_string() } else { server.auth.to_string() },
        "server_name":"localhost","certificate":certificate,"key":key}).to_string()).unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), tokio::process::Command::new(env!("CARGO_BIN_EXE_yo3-launcher"))
        .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()))
        .env("YO3_TEST_LAUNCHED", dir.join("launched")).env("YO3_TEST_SECRET", &key)
        .arg("--profile").arg(profile).arg("--game-dir").arg(&dir)
        .arg("--injector").arg(dir.join("injector.exe")).arg("--dll").arg(dir.join("auth.dll"))
        .kill_on_drop(true).output()).await.unwrap().unwrap();
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(!result.status.success());
    let stage = if runtime { "stage=runtime-dependency" } else { "stage=mod-load" };
    assert!(error.contains(stage), "Native failure must reach the desktop instead of only EOF: {error}");
    assert!(!error.contains(key.trim()), "credentials must be redacted from propagated native errors");
    assert!(!error.contains(&key.trim()[key.trim().len()-12..]), "truncating stderr must not expose a credential suffix: {error}");
    if runtime {
        assert!(!dir.join("launched").exists(), "Runtime validation must not launch the game");
        let summary = error.lines().next().unwrap();
        assert!(summary.contains("DirectX") && !summary.contains("stage="), "Normal mode needs a plain repair summary: {summary}");
    } else {
        let summary = error.lines().next().unwrap();
        assert!(summary.contains("Connection DLL initialization failed") && !summary.contains("stage="), "The native repair message should lead, with technical codes in details: {summary}");
    }
    let sessions = dir.join("logs/sessions");
    let logs: String = std::fs::read_dir(sessions).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "log"))
        .map(|p| std::fs::read_to_string(p).unwrap()).collect();
    assert!(logs.contains(stage), "The actionable native error must be in exported diagnostics");
    assert!(!logs.contains(key.trim()));
    std::fs::remove_dir_all(dir).unwrap();
}
