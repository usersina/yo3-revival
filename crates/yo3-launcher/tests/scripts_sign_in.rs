//! The Scripts edition's password sign-in (#23): `yo3-launcher --sign-in`/`--register` with a server's invite link, a
//! profile with no secret in it, and `play` resuming the saved sign-in or showing the game's own form (#34) without one.
use std::io::Write;
use std::process::{Command, Stdio};
use yo3_launcher::direct::{self, Refusal};
use yo3_launcher::signin::{self, Kept};
use yo3_launcher::vault::{MemoryVault, Vault};
use yo3_launcher::{Credential, Ports, Proxy};
use yo3_tunnel::invite::Invite;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const PASSWORD: &str = "LongPassphrase1";
/// A wrong password makes the server wait a second before the next try (the throttle, #23).
const THROTTLE: std::time::Duration = std::time::Duration::from_millis(1100);

struct Scratch { dir: std::path::PathBuf, invite: Invite, _server: yo3_server::Running, data: std::path::PathBuf, code: String }

async fn scratch(name: &str, settings: &[(&str, &str)]) -> Scratch {
    yo3_server::logins::use_fast_hashing();
    let dir = std::env::temp_dir().join(format!("yo3-scripts-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut cfg = yo3_server::Config::ephemeral(dir.join("server"));
    cfg.settings.extend(settings.iter().map(|(k, v)| (k.to_string(), v.to_string())));
    let identity = yo3_tunnel::identity_from_pem(&std::fs::read(&cfg.tls.certificate).unwrap()).unwrap();
    let data = cfg.data_dir.clone().unwrap();
    // The host's invite code, made before the server starts (a running server keeps its accounts in memory).
    std::fs::create_dir_all(&data).unwrap();
    let code = yo3_server::accounts::Directory::open(vec![], &data.join("accounts.json")).unwrap().add_invite(Some(1), None, "host").unwrap();
    let server = yo3_server::start(cfg).await.unwrap();
    let invite = Invite::new(&format!("localhost:{}", server.auth.port()), &identity, "Scratch").unwrap();
    std::fs::create_dir_all(dir.join("player")).unwrap();
    Scratch { dir, invite, _server: server, data, code }
}

/// A vault that can't be reached, as on a Linux PC without a Secret Service.
struct NoStore;
impl Vault for NoStore {
    fn get(&self, _: &str) -> anyhow::Result<Option<String>> { anyhow::bail!("the system's credential store can't be read") }
    fn set(&self, _: &str, _: &str) -> anyhow::Result<()> { anyhow::bail!("the system's credential store can't be written") }
    fn delete(&self, _: &str) -> anyhow::Result<()> { anyhow::bail!("the system's credential store can't be written") }
}

#[tokio::test]
async fn a_sign_in_keeps_the_device_token_in_the_store_and_the_profile_holds_no_secret() {
    let s = scratch("saved", &[("registration", "open")]).await;
    let endpoint = signin::endpoint(&s.invite).unwrap();
    let recovery = signin::register(&endpoint, "Scripter", PASSWORD, "", true).await.unwrap();
    assert!(!recovery.is_empty());
    let vault = MemoryVault::default();
    let wrong = signin::sign_in(&endpoint, &s.invite, "Scripter", "WrongPassword9", &vault).await.unwrap_err();
    let refused = wrong.downcast_ref::<signin::Refused>().unwrap();
    assert_eq!((refused.code.as_str(), refused.sentence.as_str()), ("invalid_credentials", "The User ID or password is wrong."));
    assert_eq!(vault.0.lock().unwrap().len(), 0);
    tokio::time::sleep(THROTTLE).await;
    assert_eq!(signin::sign_in(&endpoint, &s.invite, "Scripter", PASSWORD, &vault).await.unwrap(), Kept::Saved);
    let account = signin::vault_account(&s.invite.identity);
    assert_eq!(account, format!("server:{}", s.invite.identity), "Desktop's entry for the same server");
    assert!(vault.get(&account).unwrap().is_some());

    let profile = s.dir.join("player/client.json");
    signin::write_profile(&profile, &s.invite).unwrap();
    let text = std::fs::read_to_string(&profile).unwrap();
    assert!(!text.contains(PASSWORD) && !text.contains(&vault.get(&account).unwrap().unwrap()) && !text.contains("\"key\""), "{text}");
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; assert_eq!(std::fs::metadata(&profile).unwrap().permissions().mode() & 0o777, 0o600); }
    let loaded = yo3_launcher::Profile::read(&profile).unwrap();
    assert!(matches!(&loaded.credential, Credential::Device(a) if *a == account));
    assert_eq!(loaded.vault.as_deref(), Some(account.as_str()), "Delete Login Data forgets it");
    let read = |a: &str| vault.get(a).unwrap();
    direct::sign_in(&loaded.endpoint, &loaded.credential, &read).await.expect("play resumes the saved sign-in");

    // A key profile may be the only copy of its key: it is never overwritten.
    let keyed = s.dir.join("player/key.json");
    std::fs::write(&keyed, r#"{"server":"x:1","server_name":"x","key":"secret"}"#).unwrap();
    assert!(signin::write_profile(&keyed, &s.invite).is_err());
    assert!(std::fs::read_to_string(&keyed).unwrap().contains("secret"));
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[tokio::test]
async fn without_a_credential_store_the_sign_in_works_but_keeps_nothing() {
    let s = scratch("nostore", &[("registration", "open")]).await;
    let endpoint = signin::endpoint(&s.invite).unwrap();
    signin::register(&endpoint, "Scripter", PASSWORD, "", true).await.unwrap();
    assert!(signin::vault_problem(&NoStore).is_some() && signin::vault_problem(&MemoryVault::default()).is_none());
    assert!(matches!(signin::sign_in(&endpoint, &s.invite, "Scripter", PASSWORD, &NoStore).await.unwrap(), Kept::NotKept(_)));
    let devices = yo3_server::accounts::Directory::open(vec![], &s.data.join("accounts.json")).unwrap()
        .login_of("Scripter").unwrap().1.devices.len();
    assert_eq!(devices, 0, "no device token the store can't keep");
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[tokio::test]
async fn an_invite_only_server_takes_the_hosts_code_and_the_rules() {
    let s = scratch("invite", &[("registration", "invite")]).await;
    let code = s.code.clone();
    let endpoint = signin::endpoint(&s.invite).unwrap();
    let no_code = signin::register(&endpoint, "Scripter", PASSWORD, "", true).await.unwrap_err();
    assert_eq!(no_code.downcast_ref::<signin::Refused>().unwrap().code, "invite_required");
    let no_rules = signin::register(&endpoint, "Scripter", PASSWORD, &code, false).await.unwrap_err();
    assert_eq!(no_rules.downcast_ref::<signin::Refused>().unwrap().code, "rules_not_accepted");
    signin::register(&endpoint, "Scripter", PASSWORD, &code, true).await.unwrap();
    let _ = std::fs::remove_dir_all(&s.dir);
}

#[test]
fn the_server_is_an_invite_link_or_an_address_with_its_identity() {
    let identity = "ab".repeat(32);
    let invite = Invite::new("play.example:7444", &identity, "Friday duels").unwrap();
    assert_eq!(signin::server(&invite.link(), None).unwrap(), invite);
    assert_eq!(signin::server(&invite.link(), Some(&identity.to_uppercase())).unwrap(), invite);
    assert!(signin::server(&invite.link(), Some(&"cd".repeat(32))).is_err(), "a link and --identity that differ");
    let by_address = signin::server("play.example:7444", Some(&identity)).unwrap();
    assert_eq!((by_address.address.as_str(), by_address.identity.as_str()), ("play.example:7444", identity.as_str()));
    let error = signin::server("play.example", None).unwrap_err();
    assert!(format!("{error:#}").contains("yo3-server share"), "{error:#}");
}

async fn http(proxy: &Proxy, path: &str) -> String {
    let mut s = tokio::net::TcpStream::connect(("127.0.0.1", proxy.ports.http)).await.unwrap();
    s.write_all(format!("GET /session/{}/{path} HTTP/1.1\r\nHost: localhost\r\n\r\n", yo3_launcher::hex(&proxy.cookie)).as_bytes()).await.unwrap();
    let mut reply = vec![]; let _ = s.read_to_end(&mut reply).await;
    String::from_utf8(reply).unwrap()
}

/// `play` with a sign-in profile but no saved sign-in (never kept, or forgotten by Delete Login Data): the game's own
/// login form shows, and what is typed there signs in; with Save Password the device keeps it for the next `play`.
#[tokio::test]
async fn play_without_a_saved_sign_in_shows_the_games_form_and_signs_in_from_it() {
    let s = scratch("form", &[("registration", "open")]).await;
    let endpoint = signin::endpoint(&s.invite).unwrap();
    signin::register(&endpoint, "Scripter", PASSWORD, "", true).await.unwrap();
    let account = signin::vault_account(&s.invite.identity);
    let vault = MemoryVault::default();
    let read = |a: &str| vault.get(a).unwrap();
    let failure = direct::sign_in(&endpoint, &Credential::Device(account.clone()), &read).await.unwrap_err();
    assert_eq!(failure.refusal, Refusal::SignedOut);
    let proxy = Proxy::prepare(endpoint.clone(), Ports::default()).await.unwrap();
    proxy.refuse(failure.refusal);
    assert!(http(&proxy, yo3_launcher::LOGIN_STATE_PATH).await.ends_with("form"));
    let mut session = None;
    let bad = direct::Typed { user: "Scripter".into(), password: "WrongPassword9".into(), stay: true };
    let refused = direct::form_sign_in(&endpoint, &proxy, &bad, Some(&account), &vault, &mut session, true).await.unwrap_err();
    assert_eq!(refused.refusal, Refusal::WrongPassword);
    assert!(session.is_none() && http(&proxy, yo3_launcher::LOGIN_STATE_PATH).await.ends_with("form"), "the form shows again");
    tokio::time::sleep(THROTTLE).await;
    let typed = direct::Typed { user: "Scripter".into(), password: PASSWORD.into(), stay: true };
    direct::form_sign_in(&endpoint, &proxy, &typed, Some(&account), &vault, &mut session, true).await.unwrap();
    assert!(session.is_some() && http(&proxy, yo3_launcher::LOGIN_STATE_PATH).await.ends_with("ready"));
    assert!(vault.get(&account).unwrap().is_some(), "Save Password: the next play resumes it");
    let _ = std::fs::remove_dir_all(&s.dir);
}

/// The command itself, as a Scripts player runs it: the password from standard input (a terminal gets a prompt that
/// doesn't echo), and a credential store that isn't there (this test never reaches the desktop's own).
#[tokio::test(flavor = "multi_thread")]
async fn the_register_and_sign_in_commands_write_the_profile() {
    let s = scratch("cli", &[("registration", "invite")]).await;
    let code = s.code.clone();
    let profile = s.dir.join("player/client.json");
    let run = |args: &[&str], input: String| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_yo3-launcher")).args(args)
            .env("DBUS_SESSION_BUS_ADDRESS", format!("unix:path={}", s.dir.join("no-bus").display()))
            .env("XDG_RUNTIME_DIR", s.dir.join("player"))
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned())
    };
    let link = s.invite.link();
    let profile_arg = profile.to_str().unwrap();
    let (ok, out, err) = tokio::task::block_in_place(|| run(&["--register", &link, "Scripter", "--profile", profile_arg],
        format!("{code}\n{PASSWORD}\n{PASSWORD}\nyes\n")));
    assert!(ok, "{out}\n{err}");
    assert!(out.contains("Recovery code:"), "{out}");
    assert!(err.contains("credential store") && err.contains("login form"), "the store isn't there: said so\n{err}");
    assert!(!out.contains(PASSWORD) && !err.contains(PASSWORD));
    assert!(matches!(yo3_launcher::Profile::read(&profile).unwrap().credential, Credential::Device(_)));
    let (ok, out, err) = tokio::task::block_in_place(|| run(&["--sign-in", &link, "Scripter", "--profile", profile_arg], "WrongPassword9\n".into()));
    assert!(!ok && err.contains("The User ID or password is wrong."), "{out}\n{err}");
    tokio::time::sleep(THROTTLE).await;
    let (ok, out, err) = tokio::task::block_in_place(|| run(&["--sign-in", &link, "Scripter", "--profile", profile_arg], format!("{PASSWORD}\n")));
    assert!(ok, "{out}\n{err}");
    assert!(out.contains("play"), "it says what comes next: {out}");
    let _ = std::fs::remove_dir_all(&s.dir);
}
