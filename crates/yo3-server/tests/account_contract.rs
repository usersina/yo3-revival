//! TLS operation 6, the account requests (#23): sign-up, password and device sign-ins, sign-out, password changes and
//! recovery, each against a real server. Every server allows ten quick requests from one address (the bucket), so each
//! test stays under that.
use std::path::{Path, PathBuf};

use serde_json::{Value as Json, json};
use yo3_server::accounts::Directory;
use yo3_server::logins::{self, Login, Registration, Standing};
use yo3_server::{Config, Running, start};
use yo3_tunnel::{Endpoint, Role, Token};

struct Server { endpoint: Endpoint, data: PathBuf, _running: Running }

fn config(test: &str) -> Config {
    logins::use_fast_hashing();
    let dir = std::env::temp_dir().join(format!("yo3-account-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    Config::ephemeral(dir)
}

fn accounts_file(cfg: &Config) -> PathBuf { cfg.data_dir.clone().unwrap().join("accounts.json") }

async fn serve(cfg: Config) -> Server {
    let (certificate, data) = (cfg.tls.certificate.clone(), cfg.data_dir.clone().unwrap());
    let running = start(cfg).await.unwrap();
    let endpoint = Endpoint::new(running.auth.to_string(), "localhost".into(), &certificate).unwrap();
    Server { endpoint, data, _running: running }
}

async fn server(test: &str) -> Server { serve(config(test)).await }

impl Server {
    async fn ask(&self, request: Json) -> Json { self.endpoint.account(&request).await.unwrap() }
    async fn register(&self, user: &str, password: &str) -> Json {
        self.ask(json!({ "op": "register", "user": user, "password": password, "rules": true })).await
    }
    async fn login(&self, user: &str, password: &str) -> Json { self.ask(json!({ "op": "login", "user": user, "password": password })).await }
    fn login_of(&self, user: &str) -> Login { Directory::open(vec![], &self.data.join("accounts.json")).unwrap().login_of(user).unwrap().1 }
}

fn session(reply: &Json) -> Token {
    let hex = reply["session"].as_str().unwrap_or_else(|| panic!("no session in {reply}"));
    assert_eq!(hex.len(), 64);
    std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap())
}

/// A password account made before the server starts (as the host's `add-user` does), with `change` applied.
fn made(file: &Path, user: &str, password: &str, change: impl FnOnce(&mut Directory, &str)) {
    let mut d = Directory::open(vec![], file).unwrap();
    let login = Login::new(user, logins::hash_password_now(password).unwrap(), logins::digest("unused"));
    d.register(login, None, Registration::Open).unwrap();
    change(&mut d, user);
}

const PASSWORD: &str = "CorrectHorse42";

#[tokio::test]
async fn a_new_account_signs_in_and_its_session_gets_tickets() {
    let s = server("register").await;
    let made = s.register("Yugi", PASSWORD).await;
    assert_eq!(made["ok"], true, "{made}");
    let code = made["recovery_code"].as_str().unwrap();
    assert_eq!(logins::normalize_code(code).len(), 16, "a recovery code shown once: {code}");
    let signed = s.login("yugi", PASSWORD).await;
    assert_eq!((&signed["ok"], &signed["user"]), (&json!(true), &json!("Yugi")), "a User ID ignores case: {signed}");
    assert!(signed.get("device_token").is_none(), "no device kept unless asked");
    s.endpoint.ticket(&session(&signed), Role::Front).await.unwrap();
    // What is kept: a PHC argon2id hash and a digest of the recovery code, never the secrets themselves.
    let login = s.login_of("Yugi");
    assert!(login.password.starts_with("$argon2id$") && !login.password.contains(PASSWORD));
    assert_eq!(login.recovery, logins::digest(&logins::normalize_code(code)));
}

#[tokio::test]
async fn sign_up_checks_the_user_id_the_password_and_the_rules() {
    let s = server("sign-up").await;
    let error = |reply: Json| reply["error"].as_str().unwrap_or("ok").to_string();
    assert_eq!(error(s.register("ab", PASSWORD).await), "invalid_user_id", "3 to 16 characters");
    assert_eq!(error(s.register("bad name", PASSWORD).await), "invalid_user_id", "letters and digits");
    assert_eq!(error(s.register("Yugi_M", PASSWORD).await), "invalid_user_id", "the game's login form types no _ or -");
    assert_eq!(error(s.register("Yugi", "heart of cards").await), "password_characters", "nor spaces");
    assert_eq!(error(s.register("Yugi", "HeartOfTheCards99").await), "password_too_long", "16 at most");
    assert_eq!(error(s.register("Yugi", "short").await), "password_too_short");
    assert_eq!(error(s.register("Yugi", "password1").await), "password_too_common");
    assert_eq!(error(s.ask(json!({ "op": "register", "user": "Yugi", "password": PASSWORD })).await), "rules_not_accepted");
    assert_eq!(error(s.register("Yugi", PASSWORD).await), "ok");
    assert_eq!(error(s.register("YUGI", "OtherPass77").await), "user_id_taken", "unique ignoring case");
}

/// A sign-up whose reply was lost (both sides give up at their timeouts, the account made): the same User ID and
/// password sign up again for a while and get a new recovery code, so the player isn't left with "User ID taken" and a
/// code they never saw. The first code stops working; another password still finds the User ID taken.
#[tokio::test]
async fn a_sign_up_whose_reply_was_lost_can_be_sent_again() {
    let s = server("sign-up-again").await;
    let first = s.register("Yugi", PASSWORD).await;
    let count = || Directory::open(vec![], &s.data.join("accounts.json")).unwrap().accounts().count();
    let before = count();
    let again = s.register("yugi", PASSWORD).await;
    assert_eq!(again["ok"], true, "{again}");
    assert_eq!(again["user"], "Yugi", "the account as it was made");
    let code = again["recovery_code"].as_str().unwrap();
    assert_ne!(Some(code), first["recovery_code"].as_str(), "a new code: the first was never seen");
    assert_eq!(s.login_of("Yugi").recovery, logins::digest(&logins::normalize_code(code)));
    assert_eq!(s.register("Yugi", "OtherPass77").await["error"], "user_id_taken");
    assert_eq!(count(), before, "no second account");
}

/// Only a sign-up that has just happened can be sent again: an older account, or one the host made, is taken.
#[tokio::test]
async fn an_older_account_is_not_signed_up_again() {
    let cfg = config("sign-up-old");
    made(&accounts_file(&cfg), "Yugi", PASSWORD, |_, _| {});
    let s = serve(cfg).await;
    assert_eq!(s.register("Yugi", PASSWORD).await["error"], "user_id_taken");
}

#[tokio::test]
async fn an_invite_only_server_takes_each_code_for_its_uses() {
    let mut cfg = config("invite");
    cfg.settings.push(("registration".into(), "invite".into()));
    let code = Directory::open(vec![], &accounts_file(&cfg)).unwrap().add_invite(Some(1), None, "host").unwrap();
    let s = serve(cfg).await;
    let with = |user: &str, invite: Option<&str>| s.ask(json!({ "op": "register", "user": user, "password": PASSWORD, "rules": true, "invite": invite }));
    assert_eq!(with("Yugi", None).await["error"], "invite_required");
    assert_eq!(with("Yugi", Some("AAAA-BBBB-CCCC-DDDD")).await["error"], "invite_invalid");
    assert_eq!(with("Yugi", Some(&code.to_lowercase().replace('-', ""))).await["ok"], true, "typed in any case, without dashes");
    assert_eq!(with("Kaiba", Some(&code)).await["error"], "invite_invalid", "used up");
    let invites = Directory::open(vec![], &s.data.join("accounts.json")).unwrap().invites().to_vec();
    assert_eq!((invites[0].uses, invites[0].used_by.len()), (Some(0), 1), "who used it is recorded");
}

#[tokio::test]
async fn a_closed_server_makes_no_accounts_and_discovery_says_so() {
    let mut cfg = config("closed");
    cfg.settings.extend([("registration".into(), "closed".into()), ("server-name".into(), "Kaiba Land".into())]);
    let s = serve(cfg).await;
    assert_eq!(s.register("Yugi", PASSWORD).await["error"], "registration_closed");
    let info = s.endpoint.info().await.unwrap();
    assert_eq!((info.name.as_str(), info.registration.as_str(), info.online), ("Kaiba Land", "closed", Some(0)));
}

#[tokio::test]
async fn a_wrong_password_and_an_unknown_user_id_look_the_same_and_failures_back_off() {
    let s = server("backoff").await;
    s.register("Yugi", PASSWORD).await;
    let unknown = s.login("Nobody", PASSWORD).await;
    let wrong = s.login("Yugi", "not the password").await;
    assert_eq!(unknown, wrong, "one answer for both");
    assert_eq!(wrong["error"], "invalid_credentials");
    // Right after a failure even the right password waits: the backoff (1 s, doubling to 15 minutes) holds the User ID.
    let waiting = s.login("Yugi", PASSWORD).await;
    assert_eq!(waiting["error"], "throttled", "{waiting}");
    assert!(waiting["retry_after"].as_u64().unwrap() >= 1);
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    assert_eq!(s.login("Yugi", PASSWORD).await["ok"], true, "after the wait; a success resets the backoff");
}

#[tokio::test]
async fn one_address_gets_ten_quick_requests_then_waits() {
    let s = server("bucket").await;
    for _ in 0..10 { assert_eq!(s.ask(json!({ "op": "nothing" })).await["error"], "unknown_op") }
    let waiting = s.ask(json!({ "op": "nothing" })).await;
    assert_eq!(waiting["error"], "throttled", "{waiting}");
    assert!(waiting["retry_after"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn the_newest_login_ends_the_older_session() {
    let s = server("newest").await;
    s.register("Yugi", PASSWORD).await;
    let first = session(&s.login("Yugi", PASSWORD).await);
    let second = session(&s.login("Yugi", PASSWORD).await);
    assert!(s.endpoint.ticket(&first, Role::Front).await.is_err(), "the older launcher's session ended");
    s.endpoint.ticket(&second, Role::Front).await.unwrap();
}

#[tokio::test]
async fn a_device_stays_signed_in_until_it_signs_out() {
    let s = server("device").await;
    s.register("Yugi", PASSWORD).await;
    let signed = s.ask(json!({ "op": "login", "user": "Yugi", "password": PASSWORD, "stay": true, "device": "Windows PC\n" })).await;
    let token = signed["device_token"].as_str().unwrap().to_string();
    let kept = s.login_of("Yugi").devices;
    assert_eq!((kept.len(), kept[0].label.as_str()), (1, "Windows PC"), "labelled, control characters dropped");
    assert_eq!(kept[0].token, logins::digest(&token), "kept as a digest only");
    assert_eq!(kept[0].expires - kept[0].created, logins::DEVICE_TTL);
    let resumed = s.ask(json!({ "op": "resume", "device_token": token })).await;
    s.endpoint.ticket(&session(&resumed), Role::Front).await.unwrap();
    assert_eq!(s.ask(json!({ "op": "sign_out", "device_token": token })).await["ok"], true);
    assert_eq!(s.ask(json!({ "op": "resume", "device_token": token })).await["error"], "signed_out");
    assert_eq!(s.ask(json!({ "op": "resume", "device_token": "made up" })).await["error"], "signed_out");
}

#[tokio::test]
async fn sign_out_everywhere_ends_every_device_and_the_game() {
    let s = server("everywhere").await;
    s.register("Yugi", PASSWORD).await;
    let stay = json!({ "op": "login", "user": "Yugi", "password": PASSWORD, "stay": true });
    let laptop = s.ask(stay.clone()).await["device_token"].as_str().unwrap().to_string();
    let desktop = s.ask(stay).await;
    let playing = session(&desktop);
    assert_eq!(s.ask(json!({ "op": "sign_out", "device_token": laptop, "everywhere": true })).await["ok"], true);
    assert_eq!(s.ask(json!({ "op": "resume", "device_token": desktop["device_token"] })).await["error"], "signed_out");
    assert!(s.endpoint.ticket(&playing, Role::Front).await.is_err(), "the game's session ended too");
}

#[tokio::test]
async fn a_new_password_signs_every_device_out() {
    let s = server("change").await;
    s.register("Yugi", PASSWORD).await;
    let token = s.ask(json!({ "op": "login", "user": "Yugi", "password": PASSWORD, "stay": true })).await["device_token"].clone();
    let change = |new: &str| s.ask(json!({ "op": "change_password", "user": "Yugi", "password": PASSWORD, "new_password": new }));
    assert_eq!(change("short").await["error"], "password_too_short");
    assert_eq!(change("NewLongPass7").await["ok"], true);
    assert_eq!(s.ask(json!({ "op": "resume", "device_token": token })).await["error"], "signed_out");
    assert_eq!(s.login("Yugi", "NewLongPass7").await["ok"], true);
    assert_eq!(s.login("Yugi", PASSWORD).await["error"], "invalid_credentials");
}

#[tokio::test]
async fn a_new_password_ends_the_sessions_signed_in_with_the_old_one() {
    // Security review (#39): someone who knew the old password keeps no game session after the owner changes it.
    let s = server("change-session").await;
    s.register("Yugi", PASSWORD).await;
    let old = session(&s.login("Yugi", PASSWORD).await);
    let changed = s.ask(json!({ "op": "change_password", "user": "Yugi", "password": PASSWORD, "new_password": "NewLongPass7" })).await;
    assert_eq!(changed["ok"], true, "{changed}");
    assert!(s.endpoint.ticket(&old, Role::Front).await.is_err(), "the old session gets no more tickets");
}

#[tokio::test]
async fn a_recovered_account_ends_the_sessions_signed_in_before() {
    // Security review (#39): recovering a taken account throws the taker out of the game too, not only off devices.
    let s = server("recover-session").await;
    let code = s.register("Yugi", PASSWORD).await["recovery_code"].as_str().unwrap().to_string();
    let taker = session(&s.login("Yugi", PASSWORD).await);
    let done = s.ask(json!({ "op": "recover", "user": "Yugi", "code": code, "new_password": "NewLongPass7" })).await;
    assert_eq!(done["ok"], true, "{done}");
    assert!(s.endpoint.ticket(&taker, Role::Front).await.is_err(), "the session from before the recovery ended");
}

#[tokio::test]
async fn a_sign_up_without_a_usable_invite_hashes_no_password() {
    // Security review (#39): an invite-only server checks the invite before the costly password hash, so sign-ups
    // without one can't keep the two hashing slots busy.
    let mut cfg = config("invite-first");
    cfg.settings.push(("registration".into(), "invite".into()));
    let s = serve(cfg).await;
    let with = |password: &str, invite: Option<&str>| s.ask(json!({ "op": "register", "user": "Yugi", "password": password, "rules": true, "invite": invite }));
    assert_eq!(with("NoInvite4Me", None).await["error"], "invite_required");
    assert_eq!(with("BadInvite4Me", Some("AAAA-BBBB-CCCC-DDDD")).await["error"], "invite_invalid");
    assert!(!logins::was_hashed("NoInvite4Me") && !logins::was_hashed("BadInvite4Me"), "refused before hashing");
}

#[tokio::test]
async fn a_long_user_id_is_refused_before_the_throttle_keeps_it() {
    // Security review (#39): the per-User ID backoff keeps only names a User ID can be, not kilobytes from anyone.
    let s = server("long-user").await;
    let long = "y".repeat(4000);
    assert_eq!(s.ask(json!({ "op": "recover", "user": long, "code": "AAAA", "new_password": "NewLongPass7" })).await["error"], "invalid_recovery");
    assert_eq!(s.login(&long, PASSWORD).await["error"], "invalid_credentials");
    assert_eq!(s.ask(json!({ "op": "recover", "user": long, "code": "AAAA", "new_password": "NewLongPass7" })).await["error"], "invalid_recovery",
        "no backoff kept for a name that can't be a User ID");
}

#[tokio::test]
async fn a_recovery_code_works_once_and_a_new_one_replaces_it() {
    let s = server("recover").await;
    let code = s.register("Yugi", PASSWORD).await["recovery_code"].as_str().unwrap().to_string();
    let recover = |code: &str| s.ask(json!({ "op": "recover", "user": "Yugi", "code": code, "new_password": "NewLongPass7" }));
    let done = recover(&code.to_lowercase()).await;
    assert_eq!(done["ok"], true, "{done}");
    let next = done["recovery_code"].as_str().unwrap();
    assert_ne!(next, code);
    assert_eq!(s.login("Yugi", "NewLongPass7").await["ok"], true);
    assert_eq!(recover(&code).await["error"], "invalid_recovery", "used");
}

#[tokio::test]
async fn a_moderators_reset_code_works_once_for_a_day() {
    let cfg = config("reset");
    let code = logins::new_code();
    made(&accounts_file(&cfg), "Yugi", PASSWORD, |d, user| {
        d.issue_reset(user, logins::digest(&logins::normalize_code(&code)), logins::now() + logins::RESET_TTL).unwrap();
    });
    let before = Directory::open(vec![], &accounts_file(&cfg)).unwrap().login_of("Yugi").unwrap().1.recovery;
    let s = serve(cfg).await;
    let recover = || s.ask(json!({ "op": "recover", "user": "Yugi", "code": code, "new_password": "NewLongPass7" }));
    let done = recover().await;
    assert_eq!(done["ok"], true, "{done}");
    assert!(done.get("recovery_code").is_none(), "the player's own recovery code stays as it was");
    assert_eq!(s.login_of("Yugi").recovery, before);
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    assert_eq!(recover().await["error"], "invalid_recovery", "used");
}

#[tokio::test]
async fn a_banned_or_restricted_account_is_refused_with_the_reason() {
    let cfg = config("ban");
    made(&accounts_file(&cfg), "Yugi", PASSWORD, |d, user| {
        d.set_standing(user, Standing::Banned { reason: "cheating".into(), until: None }).unwrap();
    });
    made(&accounts_file(&cfg), "Kaiba", PASSWORD, |d, user| {
        d.set_standing(user, Standing::Restricted { reason: "chat".into(), until: None }).unwrap();
    });
    made(&accounts_file(&cfg), "Joey", PASSWORD, |d, user| {
        d.set_standing(user, Standing::Banned { reason: "a week".into(), until: Some(logins::now() - 1) }).unwrap();
    });
    let s = serve(cfg).await;
    let banned = s.login("Yugi", PASSWORD).await;
    assert_eq!((&banned["error"], &banned["reason"], &banned["until"]), (&json!("banned"), &json!("cheating"), &Json::Null));
    let restricted = s.login("Kaiba", PASSWORD).await;
    assert_eq!((&restricted["error"], &restricted["reason"]), (&json!("restricted"), &json!("chat")), "a restriction refuses the next sign-in (#26)");
    assert_eq!(s.login("Joey", PASSWORD).await["ok"], true, "a ban that ended");
}

#[tokio::test]
async fn an_older_launcher_is_told_to_update() {
    let mut cfg = config("update");
    cfg.settings.push(("min-launcher".into(), "0.2.0".into()));
    let s = serve(cfg).await;
    let old = s.ask(json!({ "op": "login", "user": "Yugi", "password": PASSWORD, "launcher": "0.1.9" })).await;
    assert_eq!((&old["error"], &old["min_launcher"]), (&json!("update_required"), &json!("0.2.0")));
    assert_eq!(s.ask(json!({ "op": "login", "user": "Yugi", "password": PASSWORD, "launcher": "v0.2.0" })).await["error"], "invalid_credentials");
    assert_eq!(s.endpoint.info().await.unwrap().min_launcher.as_deref(), Some("0.2.0"));
}

/// The host's clean-game policy (#69) is in the server's status, which the launcher reads before PLAY.
#[tokio::test]
async fn the_clean_game_setting_is_in_the_servers_status() {
    let off = serve(config("clean-off")).await;
    assert!(!off.endpoint.info().await.unwrap().clean_game, "off unless the host asks");
    let mut cfg = config("clean-on");
    cfg.settings.push(("clean-game".into(), "on".into()));
    let on = serve(cfg).await;
    assert!(on.endpoint.info().await.unwrap().clean_game);
}

#[tokio::test]
async fn omitting_or_forging_the_launcher_version_cannot_bypass_the_update_gate() {
    let s=server("version-required").await;
    for version in [None,Some("0.0.0"),Some("not-a-version")] {
        let mut request=json!({"op":"login","user":"Yugi","password":PASSWORD});
        if let Some(version)=version { request["launcher"]=json!(version); }
        let body=serde_json::to_vec(&request).unwrap();
        let mut wire=(body.len() as u16).to_be_bytes().to_vec();wire.extend(body);
        let reply=s.endpoint.raw(6,&wire).await.unwrap();
        let reply:Json=serde_json::from_slice(&reply[2..]).unwrap();
        assert_eq!(reply["error"],"update_required","{request}: {reply}");
    }
    let current=s.login("Yugi",PASSWORD).await;
    assert_eq!(current["error"],"invalid_credentials","the current launcher passes the version gate");
}

#[tokio::test]
async fn a_request_that_is_not_an_object_is_refused() {
    let s = server("bad").await;
    assert_eq!(s.ask(json!(["login"])).await["error"], "bad_request");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_most_two_passwords_are_hashed_at_once() {
    logins::use_fast_hashing();
    let hashes: Vec<_> = (0..32).map(|i| tokio::spawn(logins::hash_password(format!("password number {i}")))).collect();
    for hash in hashes { hash.await.unwrap().unwrap(); }
    let checks: Vec<_> = (0..32).map(|_| tokio::spawn(logins::verify_password(PASSWORD.into(), None))).collect();
    for check in checks { assert!(!check.await.unwrap(), "an unknown User ID never matches") }
    assert!((1..=logins::HASHING_AT_ONCE).contains(&logins::hashing_peak()), "peak {}", logins::hashing_peak());
}
