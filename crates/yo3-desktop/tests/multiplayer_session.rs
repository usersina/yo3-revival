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
    mp::register(&server, "Yugi", "LongPassphrase1", "LongPassphrase1", "", true).await.unwrap();
    let signed = mp::sign_in(&server, "Yugi", "LongPassphrase1", false).await.unwrap();
    assert_eq!(mp::session_alive(&server, &signed.session).await, Ok(()), "a fresh sign-in plays");
    let admin = |op: Op| { let live = admin::running(&data).unwrap(); tokio::task::spawn_blocking(move || api::call(&live, &op)) };
    admin(Op::Kick { who: "Yugi".into() }).await.unwrap().unwrap();
    assert_eq!(mp::session_alive(&server, &signed.session).await, Err(Problem::SignInAgain), "kicked: sign in again");
    // A newer sign-in elsewhere ends this one too (the newest wins).
    let first = mp::sign_in(&server, "Yugi", "LongPassphrase1", false).await.unwrap();
    let _second = mp::sign_in(&server, "Yugi", "LongPassphrase1", false).await.unwrap();
    assert_eq!(mp::session_alive(&server, &first.session).await, Err(Problem::SignInAgain));
    let live=admin::running(&data).unwrap();
    tokio::task::spawn_blocking(move||api::stop(&live)).await.unwrap().unwrap();
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
    let pass = "LongPassphrase1";
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
    assert_eq!(mp::change_password(&server, "Yugi", pass, "NewerPassword2", "a different one").await, Err(Problem::PasswordsDiffer));
    assert_eq!(mp::change_password(&server, "Yugi", "wrong password", "NewerPassword2", "NewerPassword2").await, Err(Problem::WrongUserOrPassword));
    throttled().await;
    mp::change_password(&server, "Yugi", pass, "NewerPassword2", "NewerPassword2").await.unwrap();
    assert_eq!(mp::resume(&server, laptop.device_token.as_deref().unwrap()).await.map(|_| ()), Err(Problem::SignInAgain));
    assert_eq!(mp::sign_in(&server, "Yugi", pass, false).await.map(|_| ()), Err(Problem::WrongUserOrPassword));
    throttled().await;
    mp::sign_in(&server, "Yugi", "NewerPassword2", false).await.unwrap();
}

/// A server that takes the connection and never answers (#65's review): an account request gives up with Unreachable
/// within its one deadline, not after the request's, then each probe's own 10 s (about 40 s before).
#[tokio::test(start_paused = true)]
async fn a_server_that_never_answers_is_unreachable_within_the_account_deadline() {
    let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = silent.local_addr().unwrap().to_string();
    std::thread::spawn(move || { let held: Vec<_> = silent.incoming().collect(); drop(held) });
    let server = mp::Server { name: "Silent".into(), address, server_name: "localhost".into(), identities: vec!["a".repeat(64)],
        official: false, description: String::new(), saved_user: None, last_used: 0 };
    let started = tokio::time::Instant::now();
    assert_eq!(mp::sign_in(&server, "Yugi", "LongPassphrase1", false).await.map(|_| ()), Err(Problem::Unreachable));
    let waited = started.elapsed();
    assert!(waited <= mp::ACCOUNT_DEADLINE + std::time::Duration::from_millis(100), "the spinner waited {waited:?}");
}

/// One dropped status probe (#65's review): the server's row still says Online, as the probe is tried once more.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_dropped_status_probe_still_shows_the_server_online() {
    let root = std::env::temp_dir().join(format!("yo3-desktop-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let running = yo3_server::start(Config::ephemeral(root.join("host"))).await.unwrap();
    let server = mp::first_contact(&format!("localhost:{}", running.auth.port())).await.unwrap();
    // A relay in front of the server that drops the first connection it gets, then carries the others.
    let relay = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relayed = mp::Server { address: relay.local_addr().unwrap().to_string(), ..server };
    let target = running.auth;
    tokio::spawn(async move {
        let (first, _) = relay.accept().await.unwrap();
        drop(first);
        loop {
            let (mut game, _) = relay.accept().await.unwrap();
            tokio::spawn(async move {
                let mut real = tokio::net::TcpStream::connect(target).await.unwrap();
                let _ = tokio::io::copy_bidirectional(&mut game, &mut real).await;
            });
        }
    });
    assert!(matches!(mp::status(&relayed).await, mp::Status::Online { .. }), "one dropped probe isn't Offline");
}

/// Report a problem, signed in on Multiplayer: the logs go to the server's host and come back as a report ID for the
/// GitHub form (problem reports).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_a_problem_gets_a_report_id_from_the_server() {
    yo3_server::logins::use_fast_hashing();
    let root = std::env::temp_dir().join(format!("yo3-desktop-problem-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut cfg = Config::ephemeral(root.join("host"));
    cfg.settings.push(("registration".into(), "open".into()));
    let data = cfg.data_dir.clone().unwrap();
    let running = yo3_server::start(cfg).await.unwrap();
    let server = mp::first_contact(&format!("localhost:{}", running.auth.port())).await.unwrap();
    mp::register(&server, "Yugi", "LongPassphrase1", "LongPassphrase1", "", true).await.unwrap();
    let signed = mp::sign_in(&server, "Yugi", "LongPassphrase1", false).await.unwrap();
    let id = mp::problem_report(&server, &signed.session, "The shop disconnected\nwhile buying", "", "== The app ==\n1 a log line").await.unwrap();
    let report = yo3_server::problems::Problems::new(Some(&data)).get(&id).unwrap();
    assert_eq!((report.kind.as_str(), report.summary.as_str(), report.account.as_str()), ("report", "The shop disconnected", "Yugi"));
    assert!(report.logs.contains("== What the player wrote ==\nThe shop disconnected\nwhile buying") && report.logs.contains("1 a log line"));
    assert_eq!(mp::problem_report(&server, &[9; 32], "x", "", "").await.map(|_| ()), Err(Problem::Other("the sign-in ended".into())));
}
