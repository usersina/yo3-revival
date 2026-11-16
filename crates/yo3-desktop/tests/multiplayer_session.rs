//! Multiplayer's Play checks a plain sign-in's session first (#30): one that ended elsewhere (a kick, a ban, the same
//! account signing in on another PC) asks the player to sign in again instead of starting a game that can't connect.
use yo3_desktop::multiplayer::{self as mp, Problem};
use yo3_server::{Config, admin, api::{self, Op}};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_ended_elsewhere_asks_to_sign_in_again() {
    yo3_server::logins::use_fast_hashing();
    let root = std::env::temp_dir().join(format!("yo3-desktop-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut cfg = Config::ephemeral(root.join("host"));
    cfg.settings.push(("registration".into(), "open".into()));
    let data = cfg.data_dir.clone().unwrap();
    let running = yo3_server::start(cfg).await.unwrap();
    let server = mp::first_contact(&format!("localhost:{}", running.auth.port())).await.unwrap();
    mp::register(&server, "Yugi", "a long passphrase", "a long passphrase", "", true).await.unwrap();
    let signed = mp::sign_in(&server, "Yugi", "a long passphrase", false).await.unwrap();
    assert_eq!(mp::session_alive(&server, &signed.session).await, Ok(()), "a fresh sign-in plays");
    let admin = |op: Op| { let live = admin::running(&data).unwrap(); tokio::task::spawn_blocking(move || api::call(&live, &op)) };
    admin(Op::Kick { who: "Yugi".into() }).await.unwrap().unwrap();
    assert_eq!(mp::session_alive(&server, &signed.session).await, Err(Problem::SignInAgain), "kicked: sign in again");
    // A newer sign-in elsewhere ends this one too (the newest wins).
    let first = mp::sign_in(&server, "Yugi", "a long passphrase", false).await.unwrap();
    let _second = mp::sign_in(&server, "Yugi", "a long passphrase", false).await.unwrap();
    assert_eq!(mp::session_alive(&server, &first.session).await, Err(Problem::SignInAgain));
    admin(Op::Shutdown).await.unwrap().unwrap();
    running.wait().await;
    assert_eq!(mp::session_alive(&server, &first.session).await, Err(Problem::Unreachable), "no answer is not a sign-out");
}

/// Settings → Accounts (#31): Sign out everywhere revokes every stay-signed-in and ends the sessions, even from a PC
/// that didn't stay signed in; Change password signs every device out and the new password signs in.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sign_out_everywhere_and_a_new_password_end_every_sign_in() {
    yo3_server::logins::use_fast_hashing();
    let root = std::env::temp_dir().join(format!("yo3-desktop-accounts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut cfg = Config::ephemeral(root.join("host"));
    cfg.settings.push(("registration".into(), "open".into()));
    cfg.settings.push(("address-burst".into(), "100".into())); // many requests from this one address
    let running = yo3_server::start(cfg).await.unwrap();
    let server = mp::first_contact(&format!("localhost:{}", running.auth.port())).await.unwrap();
    let pass = "a long passphrase";
    mp::register(&server, "Yugi", pass, pass, "", true).await.unwrap();
    let laptop = mp::sign_in(&server, "Yugi", pass, true).await.unwrap();
    let here = mp::sign_in(&server, "Yugi", pass, false).await.unwrap();
    assert!(here.device_token.is_none());
    assert_eq!(mp::sign_out_everywhere(&server, None, "Yugi", "wrong password").await, Err(Problem::WrongUserOrPassword));
    let throttled = || tokio::time::sleep(std::time::Duration::from_millis(1100)); // the first failure's wait
    throttled().await;
    mp::sign_out_everywhere(&server, None, "Yugi", pass).await.unwrap();
    assert_eq!(mp::resume(&server, laptop.device_token.as_deref().unwrap()).await.map(|_| ()), Err(Problem::SignInAgain), "the other PC is signed out");
    let laptop = mp::sign_in(&server, "Yugi", pass, true).await.unwrap();
    assert_eq!(mp::change_password(&server, "Yugi", pass, "a newer passphrase", "a different one").await, Err(Problem::PasswordsDiffer));
    assert_eq!(mp::change_password(&server, "Yugi", "wrong password", "a newer passphrase", "a newer passphrase").await, Err(Problem::WrongUserOrPassword));
    throttled().await;
    mp::change_password(&server, "Yugi", pass, "a newer passphrase", "a newer passphrase").await.unwrap();
    assert_eq!(mp::resume(&server, laptop.device_token.as_deref().unwrap()).await.map(|_| ()), Err(Problem::SignInAgain));
    assert_eq!(mp::sign_in(&server, "Yugi", pass, false).await.map(|_| ()), Err(Problem::WrongUserOrPassword));
    throttled().await;
    mp::sign_in(&server, "Yugi", "a newer passphrase", false).await.unwrap();
}
