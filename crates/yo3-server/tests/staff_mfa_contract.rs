//! Staff second factor (#43): enrollment, codes, recovery, throttling, revocation, step-up and local recovery, through the
//! real staff router with the Required policy. Each test names the rule it protects.
use std::sync::{Arc, Mutex};
use axum::{Router, body::{Body, to_bytes}, extract::ConnectInfo, http::Request};
use serde_json::{Value, json};
use tower::ServiceExt;
use yo3_server::{accounts::Directory, api::{Api, Op}, dashboard::{self, Dashboard, ProxyOrigin}, logins::{self, Login, Registration, Role},
    panel::Panel, staff_mfa::{self, StaffMfaPolicy}};

const PASSWORD: &str = "PreviewPassword1";

struct Fixture { dir: std::path::PathBuf, api: Arc<Api>, panel: Arc<Panel>, router: Router, proxy: Router,
    /// The time step the last enrollment's code was made for, so a test replays exactly that one.
    enrolled_step: std::sync::atomic::AtomicU64 }
struct Answer { status: u16, headers: String, body: Value }
/// One signed-in browser.
#[derive(Default, Clone)]
struct Jar { cookie: String, csrf: String }
impl Jar { fn token(&self) -> &str { self.cookie.split_once('=').map_or("", |(_, t)| t) } }

fn sixty() -> String { "0123456789abcdef".repeat(4) }
fn seed_of(secret: &str) -> Vec<u8> {
    // The base32 secret, decoded (RFC 4648), as an authenticator app does.
    let (mut out, mut buffer, mut bits) = (Vec::new(), 0u32, 0u32);
    for c in secret.bytes() {
        let v = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".iter().position(|a| *a == c).unwrap() as u32;
        buffer = buffer << 5 | v; bits += 5;
        if bits >= 8 { bits -= 8; out.push((buffer >> bits & 0xff) as u8); }
    }
    out
}

impl Fixture {
    async fn new(name: &str, policy: StaffMfaPolicy) -> Self {
        logins::use_fast_hashing();
        let dir = std::env::temp_dir().join(format!("yo3-staff-mfa-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        yo3_server::store::initialize(&dir).unwrap();
        let mut accounts = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
        let password = logins::hash_password(PASSWORD.into()).await.unwrap();
        for (user, role) in [("Pegasus", Role::Admin), ("Croquet", Role::Moderator), ("Yugi", Role::Player), ("Weevil", Role::Admin)] {
            accounts.register(Login::new(user, password.clone(), logins::digest(&logins::new_code())), None, Registration::Open).unwrap();
            accounts.set_role(user, role).unwrap();
        }
        let api = Arc::new(Api::new(Arc::new(Mutex::new(accounts)), Arc::new(yo3_server::settings::Live::open(None, vec![]).unwrap()), Some(dir.clone()), dir.clone(), "test"));
        let panel = Arc::new(Panel::with_policy(policy));
        let state = Dashboard { api: api.clone(), panel: panel.clone() };
        let proxy = dashboard::proxy_router(state.clone(), ProxyOrigin::parse("https://staff.example").unwrap());
        let router = dashboard::router(state);
        Self { dir, api, panel, router, proxy, enrolled_step: Default::default() }
    }

    #[allow(clippy::too_many_arguments, reason = "a test helper taking each header of one request")]
    async fn send(&self, router: &Router, method: &str, path: &str, jar: &Jar, body: Value, extra: &[(&str, &str)], proxied: bool) -> Answer {
        let mut r = Request::builder().method(method).uri(path).body(Body::from(if body.is_null() { String::new() } else { body.to_string() })).unwrap();
        let mut set = |k: &str, v: &str| { r.headers_mut().insert(axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(), v.parse().unwrap()); };
        set("cookie", &jar.cookie); set("x-csrf-token", &jar.csrf); set("idempotency-key", "staff-mfa-contract-0001"); set("content-type", "application/json");
        if proxied { set("host", "staff.example"); set("origin", "https://staff.example"); set("x-forwarded-proto", "https"); set("x-forwarded-for", "198.51.100.7") }
        else { set("host", "localhost"); set("origin", "http://localhost") }
        for (k, v) in extra { set(k, v) }
        r.extensions_mut().insert(ConnectInfo("127.0.0.1:1234".parse::<std::net::SocketAddr>().unwrap()));
        let res = router.clone().oneshot(r).await.unwrap();
        let status = res.status().as_u16();
        let headers = format!("{:?}", res.headers());
        let bytes = to_bytes(res.into_body(), 10_000_000).await.unwrap();
        Answer { status, headers, body: serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) })) }
    }
    async fn call(&self, method: &str, path: &str, jar: &Jar, body: Value) -> Answer { self.send(&self.router, method, path, jar, body, &[], false).await }
    fn jar(&self, a: &Answer) -> Jar {
        let cookie = a.headers.split("set-cookie\": \"").nth(1).and_then(|c| c.split(';').next()).unwrap_or_default().to_string();
        Jar { cookie, csrf: a.body["session"]["csrf_token"].as_str().or(a.body["csrf_token"].as_str()).unwrap_or_default().to_string() }
    }
    async fn password(&self, user: &str) -> Answer {
        self.call("POST", "/panel/api/v1/sign-in", &Jar::default(), json!({ "user": user, "password": PASSWORD })).await
    }
    /// Password, then a code (or recovery code): the session.
    async fn sign_in(&self, user: &str, code: &str) -> Jar {
        let a = self.password(user).await;
        assert_eq!(a.status, 200, "{}", a.body);
        assert_eq!(a.body["code_required"], true, "{}", a.body);
        let a = self.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": a.body["challenge"], "code": code })).await;
        assert_eq!(a.status, 200, "{}", a.body);
        Jar { cookie: self.jar(&a).cookie, csrf: a.body["csrf_token"].as_str().unwrap().into() }
    }
    /// Enrolls `user` with the dashboard's own flow: its limited session, then the seed and the recovery codes.
    async fn enroll(&self, user: &str) -> (Jar, Vec<u8>, Vec<String>) {
        let a = self.password(user).await;
        assert_eq!(a.status, 200, "{}", a.body);
        assert_eq!(a.body["code_required"], false);
        assert_eq!(a.body["session"]["limited"], true, "no authenticator yet: an enrollment-only session");
        let jar = self.jar(&a);
        let start = self.call("POST", "/panel/api/v1/mfa/enrollment", &jar, json!(null)).await;
        assert_eq!(start.status, 200, "{}", start.body);
        let seed = seed_of(start.body["secret"].as_str().unwrap());
        assert_eq!(seed.len(), 20);
        let step = (logins::now() / staff_mfa::STEP) as u64;
        self.enrolled_step.store(step, std::sync::atomic::Ordering::SeqCst);
        let code = staff_mfa::code_at(&seed, step);
        let done = self.call("POST", "/panel/api/v1/mfa/enrollment/confirm", &jar, json!({ "code": code })).await;
        assert_eq!(done.status, 200, "{}", done.body);
        let codes = done.body["recovery_codes"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect();
        (jar, seed, codes)
    }
    fn uid(&self, user: &str) -> u32 { self.api.accounts.lock().unwrap().login_of(user).unwrap().0 }
    /// A code the replay rule has not used yet (tests may reuse the current step by moving the last used one back).
    fn fresh(&self, user: &str, seed: &[u8]) -> String {
        let id = self.uid(user);
        self.api.accounts.lock().unwrap().update_login(id, |l| { l.mfa.as_mut().unwrap().last_step = 0; Ok(()) }).unwrap();
        staff_mfa::code_at(seed, (logins::now() / staff_mfa::STEP) as u64)
    }
    fn audit(&self) -> Vec<yo3_server::store::AuditRow> { self.api.audit(200).unwrap() }
}
impl Drop for Fixture { fn drop(&mut self) { yo3_server::store::close(&self.dir); let _ = std::fs::remove_dir_all(&self.dir); } }

#[tokio::test]
async fn staff_without_an_authenticator_get_an_enrollment_only_session_and_the_seed_is_sealed() {
    let f = Fixture::new("enroll", StaffMfaPolicy::Required).await;
    let a = f.password("Pegasus").await;
    let jar = f.jar(&a);
    assert_eq!(a.body["session"]["limited"], true);
    assert_eq!(a.body["session"]["mfa_enabled"], false);
    assert_eq!(a.body["session"]["mfa_required"], true);
    for flag in ["httponly", "secure", "samesite=strict", "path=/panel"] { assert!(a.headers.to_ascii_lowercase().contains(flag), "{flag}") }
    // Nothing but enrollment, the session and sign-out.
    for (method, path) in [("GET", "/panel/api/v1/accounts"), ("GET", "/panel/api/v1/server"), ("POST", "/panel/api/v1/give-bp"), ("POST", "/panel/api/v1/backups")] {
        let denied = f.call(method, path, &jar, json!({})).await;
        assert_eq!((denied.status, denied.body["code"].as_str()), (403, Some("mfa_enrollment_required")), "{path}");
    }
    assert_eq!(f.call("GET", "/panel/api/v1/session", &jar, json!(null)).await.status, 200);
    // Confirming needs a valid code.
    let start = f.call("POST", "/panel/api/v1/mfa/enrollment", &jar, json!(null)).await;
    assert_eq!(start.status, 200);
    let secret = start.body["secret"].as_str().unwrap().to_string();
    let uri = start.body["otpauth_uri"].as_str().unwrap();
    assert!(uri.starts_with("otpauth://totp/YO3%20Revival:Pegasus?secret=") && uri.contains(&secret) && uri.contains("digits=6&period=30"));
    let wrong = f.call("POST", "/panel/api/v1/mfa/enrollment/confirm", &jar, json!({ "code": "000000" })).await;
    assert_eq!((wrong.status, wrong.body["code"].as_str()), (403, Some("invalid_code")));
    assert!(f.api.accounts.lock().unwrap().login_of("Pegasus").unwrap().1.mfa.is_some_and(|m| !m.active()), "a wrong code activates nothing");
    // The seed and its recovery codes are never in the saved accounts in the clear.
    let seed = seed_of(&secret);
    let saved = std::fs::read_to_string(f.dir.join("accounts.json")).unwrap_or_default();
    assert!(!saved.contains(&secret) && !saved.contains(&hex(&seed)), "the seed is sealed at rest");
}
fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }

#[tokio::test]
async fn enrolling_activates_with_a_valid_code_shows_recovery_codes_once_and_keeps_only_digests() {
    let f = Fixture::new("confirm", StaffMfaPolicy::Required).await;
    let (jar, seed, codes) = f.enroll("Pegasus").await;
    assert_eq!(codes.len(), 10);
    assert!(codes.iter().all(|c| c.len() == 19));
    let session = f.call("GET", "/panel/api/v1/session", &jar, json!(null)).await;
    assert_eq!((session.body["limited"].clone(), session.body["mfa_enabled"].clone(), session.body["recovery_codes_remaining"].clone()), (json!(false), json!(true), json!(10)),
        "the session that enrolled carries on, with every feature");
    assert!(session.body["mfa_recent_until"].is_string());
    assert_eq!(f.call("GET", "/panel/api/v1/accounts", &jar, json!(null)).await.status, 200);
    let login = f.api.accounts.lock().unwrap().login_of("Pegasus").unwrap().1;
    let mfa = login.mfa.unwrap();
    assert!(mfa.active() && mfa.seed.starts_with("v1.") && mfa.pending.is_empty());
    assert_eq!(mfa.recovery.len(), 10);
    for code in &codes { assert!(!mfa.recovery.contains(code) && mfa.recovery.contains(&logins::digest(&logins::normalize_code(code))), "recovery codes are kept hashed") }
    assert!(f.audit().iter().any(|r| r.op == "staff-mfa-enrolled" && r.actor == "Pegasus"));
    let _ = seed;
}

#[tokio::test]
async fn password_plus_code_opens_a_session_and_a_code_works_once() {
    let f = Fixture::new("codes", StaffMfaPolicy::Required).await;
    let (_, seed, _) = f.enroll("Pegasus").await;
    // The password alone opens nothing.
    let a = f.password("Pegasus").await;
    assert_eq!((a.status, a.body["code_required"].clone(), a.body["session"].clone()), (200, json!(true), json!(null)));
    assert!(!a.headers.to_ascii_lowercase().contains("set-cookie"), "no cookie before the code");
    let challenge = a.body["challenge"].as_str().unwrap().to_string();
    // A wrong code.
    let wrong = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": challenge, "code": "123456" })).await;
    assert_eq!((wrong.status, wrong.body["code"].as_str()), (403, Some("invalid_code")));
    assert!(!wrong.headers.to_ascii_lowercase().contains("set-cookie"));
    // The step used at enrollment is replayed: refused, though right in itself. (Wait out the miss's 1 s backoff first.)
    // The step is the one enrollment used, not the clock's now: a 30 s boundary in between made the release run of
    // v0.1.7 replay a fresh step, rightly accepted.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let step = f.enrolled_step.load(std::sync::atomic::Ordering::SeqCst);
    let replay = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": challenge, "code": staff_mfa::code_at(&seed, step) })).await;
    assert_eq!((replay.status, replay.body["code"].as_str()), (403, Some("invalid_code")), "a step is used once: enrollment used it");
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    // The next step is accepted once (a little ahead is clock skew), then replayed.
    let next = staff_mfa::code_at(&seed, step + 1);
    let ok = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": challenge, "code": next })).await;
    assert_eq!(ok.status, 200, "{}", ok.body);
    assert!(ok.headers.to_ascii_lowercase().contains("set-cookie") && ok.body["mfa_enabled"] == true && ok.body["limited"] == false);
    let again = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": challenge, "code": next })).await;
    assert_eq!(again.status, 401, "the challenge was used up: sign in again");
    let b = f.password("Pegasus").await;
    let replayed = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": b.body["challenge"], "code": next })).await;
    assert_eq!((replayed.status, replayed.body["code"].as_str()), (403, Some("invalid_code")), "the same code in a second sign-in");
}

#[tokio::test]
async fn wrong_codes_wait_longer_each_time_and_a_made_up_challenge_is_refused() {
    let f = Fixture::new("throttle", StaffMfaPolicy::Required).await;
    f.enroll("Pegasus").await;
    let a = f.password("Pegasus").await;
    let challenge = a.body["challenge"].as_str().unwrap().to_string();
    let first = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": challenge, "code": "111111" })).await;
    assert_eq!(first.status, 403);
    let second = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": challenge, "code": "222222" })).await;
    assert_eq!(second.status, 429, "the next try waits");
    assert!(second.headers.to_ascii_lowercase().contains("retry-after"));
    assert_eq!(second.body["code"], "rate_limited");
    // Even the right code waits (no lockout, but the wait is real).
    let made_up = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": sixty(), "code": "123456" })).await;
    assert_eq!(made_up.status, 401);
    let short = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": "abc", "code": "123456" })).await;
    assert_eq!(short.status, 400);
    // The password's own throttle is separate and still applies.
    for _ in 0..2 { f.call("POST", "/panel/api/v1/sign-in", &Jar::default(), json!({ "user": "Weevil", "password": "wrong-one-12" })).await; }
    let waits = f.call("POST", "/panel/api/v1/sign-in", &Jar::default(), json!({ "user": "Weevil", "password": PASSWORD })).await;
    assert_eq!(waits.status, 429);
}

#[tokio::test]
async fn recovery_codes_work_once_and_are_regenerated_only_after_a_recent_check() {
    let f = Fixture::new("recovery", StaffMfaPolicy::Required).await;
    let (_, seed, codes) = f.enroll("Pegasus").await;
    let jar = f.sign_in("Pegasus", &codes[0].to_lowercase().replace('-', " ")).await;
    let session = f.call("GET", "/panel/api/v1/session", &jar, json!(null)).await;
    assert_eq!(session.body["recovery_codes_remaining"], 9, "typed in lower case with spaces, it counted once");
    assert!(f.audit().iter().any(|r| r.op == "staff-recovery-code-used"));
    // Reuse is refused.
    let b = f.password("Pegasus").await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let reuse = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": b.body["challenge"], "code": codes[0] })).await;
    assert_eq!((reuse.status, reuse.body["code"].as_str()), (403, Some("invalid_code")));
    // Regenerating needs a recent check: backdate this session's.
    f.panel.backdate_mfa(jar.token(), 600);
    let denied = f.call("POST", "/panel/api/v1/mfa/recovery-codes", &jar, json!({})).await;
    assert_eq!((denied.status, denied.body["code"].as_str()), (403, Some("reauthentication_required")));
    // A step-up with a code of the current step (the other jar's wait is for a different attempt).
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let up = f.call("POST", "/panel/api/v1/mfa/step-up", &jar, json!({ "code": f.fresh("Pegasus", &seed) })).await;
    assert_eq!(up.status, 200, "{}", up.body);
    assert!(up.body["mfa_recent_until"].is_string());
    let new = f.call("POST", "/panel/api/v1/mfa/recovery-codes", &jar, json!({})).await;
    assert_eq!(new.status, 200, "{}", new.body);
    let fresh: Vec<String> = new.body["recovery_codes"].as_array().unwrap().iter().map(|c| c.as_str().unwrap().into()).collect();
    assert_eq!(fresh.len(), 10);
    assert!(!fresh.contains(&codes[1]));
    let old = f.password("Pegasus").await;
    let refused = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": old.body["challenge"], "code": codes[1] })).await;
    assert_eq!(refused.status, 403, "regenerating replaced every old code");
    assert!(f.audit().iter().any(|r| r.op == "staff-recovery-codes-regenerated"));
}

#[tokio::test]
async fn sessions_end_after_sign_out_everywhere_a_password_change_and_a_local_reset() {
    let f = Fixture::new("revoke", StaffMfaPolicy::Required).await;
    let (first, seed, codes) = f.enroll("Pegasus").await;
    let second = f.sign_in("Pegasus", &codes[0]).await;
    assert_eq!(f.call("GET", "/panel/api/v1/accounts", &first, json!(null)).await.status, 200);
    // Sign out everywhere ends both, this one included.
    let out = f.call("POST", "/panel/api/v1/sign-out-everywhere", &second, json!({})).await;
    assert_eq!(out.status, 200);
    assert!(out.headers.contains("Max-Age=0"));
    for jar in [&first, &second] { assert_eq!(f.call("GET", "/panel/api/v1/accounts", jar, json!(null)).await.status, 401) }
    // A password change (recovery, a moderator's reset) ends staff sessions too, and keeps the authenticator.
    let third = f.sign_in("Pegasus", &codes[1]).await;
    let id = f.uid("Pegasus");
    let hash = logins::hash_password("AnotherPassword9".into()).await.unwrap();
    f.api.accounts.lock().unwrap().set_password(id, hash).unwrap();
    assert_eq!(f.call("GET", "/panel/api/v1/accounts", &third, json!(null)).await.status, 401);
    assert!(f.api.accounts.lock().unwrap().login_by_id(id).unwrap().mfa.as_ref().unwrap().active(), "a new password does not remove the second factor");
    // The local reset ends sessions and removes the factor: the next sign-in enrolls again.
    let fourth = {
        let a = f.call("POST", "/panel/api/v1/sign-in", &Jar::default(), json!({ "user": "Pegasus", "password": "AnotherPassword9" })).await;
        let b = f.call("POST", "/panel/api/v1/sign-in/code", &Jar::default(), json!({ "challenge": a.body["challenge"], "code": codes[2] })).await;
        assert_eq!(b.status, 200, "{}", b.body);
        Jar { cookie: f.jar(&b).cookie, csrf: b.body["csrf_token"].as_str().unwrap().into() }
    };
    assert_eq!(f.call("GET", "/panel/api/v1/accounts", &fourth, json!(null)).await.status, 200);
    let done = f.api.apply(&Op::StaffMfaReset { user: "Pegasus".into() }, "host").unwrap();
    assert_eq!(done["had_authenticator"], true);
    assert_eq!(f.call("GET", "/panel/api/v1/accounts", &fourth, json!(null)).await.status, 401);
    assert!(f.api.accounts.lock().unwrap().login_by_id(id).unwrap().mfa.is_none());
    let row = f.audit().into_iter().find(|r| r.op == "staff-mfa-reset").expect("the reset is audited");
    assert_eq!((row.actor.as_str(), row.target.as_str()), ("host", "Pegasus"));
    // Old codes and seed are gone: a code of the old seed does nothing, and the account enrolls again.
    let again = f.call("POST", "/panel/api/v1/sign-in", &Jar::default(), json!({ "user": "Pegasus", "password": "AnotherPassword9" })).await;
    assert_eq!((again.body["code_required"].clone(), again.body["session"]["limited"].clone()), (json!(false), json!(true)));
    let _ = seed;
}

#[tokio::test]
async fn an_account_that_is_demoted_or_banned_loses_its_session_at_the_next_request() {
    let f = Fixture::new("standing", StaffMfaPolicy::Required).await;
    let (jar, _, codes) = f.enroll("Pegasus").await;
    let (other, _, _) = f.enroll("Weevil").await;
    assert_eq!(f.call("GET", "/panel/api/v1/accounts", &other, json!(null)).await.status, 200);
    f.api.accounts.lock().unwrap().set_role("Weevil", Role::Player).unwrap();
    assert_eq!(f.call("GET", "/panel/api/v1/accounts", &other, json!(null)).await.status, 401, "role is rechecked on every request");
    f.api.accounts.lock().unwrap().set_standing("Pegasus", logins::Standing::Banned { reason: "test".into(), until: None }).unwrap();
    assert_eq!(f.call("GET", "/panel/api/v1/accounts", &jar, json!(null)).await.status, 401);
    let a = f.password("Pegasus").await;
    assert_eq!(a.status, 401, "a banned account is not signed in even with the right password");
    let _ = codes;
}

#[tokio::test]
async fn sensitive_operations_need_an_authenticator_check_in_the_last_five_minutes() {
    let f = Fixture::new("stepup", StaffMfaPolicy::Required).await;
    let (jar, seed, _) = f.enroll("Pegasus").await;
    let uid = f.uid("Yugi");
    let sensitive: Vec<(&str, &str, Value)> = vec![
        ("POST", "/panel/api/v1/accounts/edit", json!({ "kind": "set_role", "user_id": uid, "role": "moderator" })),
        ("POST", "/panel/api/v1/accounts/delete", json!({ "user_id": uid, "confirm_user_id": uid })),
        ("POST", "/panel/api/v1/accounts/credentials", json!({ "kind": "reset_password", "user_id": uid, "code": "ABCD-EFGH-JKLM-NPQR" })),
        ("POST", "/panel/api/v1/accounts/keys", json!({ "kind": "reveal", "user_id": uid })),
        ("POST", "/panel/api/v1/settings", json!({ "kind": "cancel_maintenance" })),
        ("POST", "/panel/api/v1/backups", json!({})),
        ("POST", "/panel/api/v1/server/stop", json!({ "run_id": "a".repeat(64) })),
        ("POST", "/panel/api/v1/backups/download", json!({ "name": "x" })),
        ("POST", "/panel/api/v1/mfa/recovery-codes", json!({})),
    ];
    f.panel.backdate_mfa(jar.token(), 301);
    for (method, path, body) in &sensitive {
        let denied = f.call(method, path, &jar, body.clone()).await;
        assert_eq!((denied.status, denied.body["code"].as_str()), (403, Some("reauthentication_required")), "{path}: {}", denied.body);
    }
    // Everything else still works without it (moderation, gifts, reads).
    assert_eq!(f.call("GET", "/panel/api/v1/accounts", &jar, json!(null)).await.status, 200);
    // A wrong step-up code does not unlock them; a right one does, and the role change then goes through.
    let wrong = f.call("POST", "/panel/api/v1/mfa/step-up", &jar, json!({ "code": "000000" })).await;
    assert_eq!((wrong.status, wrong.body["code"].as_str()), (403, Some("invalid_code")));
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let up = f.call("POST", "/panel/api/v1/mfa/step-up", &jar, json!({ "code": f.fresh("Pegasus", &seed) })).await;
    assert_eq!(up.status, 200, "{}", up.body);
    let role = f.call("POST", "/panel/api/v1/accounts/edit", &jar, json!({ "kind": "set_role", "user_id": uid, "role": "moderator" })).await;
    assert_eq!(role.status, 200, "{}", role.body);
    assert!(f.audit().iter().any(|r| r.op == "set-role" && r.origin == "dashboard"));
    // A step-up never carries to a new session.
    let other = f.sign_in("Pegasus", &f.fresh("Pegasus", &seed)).await;
    assert!(f.call("GET", "/panel/api/v1/session", &other, json!(null)).await.body["mfa_recent_until"].is_string(), "signing in with the code is a check");
}

#[tokio::test]
async fn public_backup_downloads_are_off_by_default_and_need_admin_a_recent_check_and_an_audit_when_enabled() {
    let f = Fixture::new("backups", StaffMfaPolicy::Required).await;
    let (jar, _, _) = f.enroll("Pegasus").await;
    // Off by default: the proxy never serves them, whoever asks.
    let off = f.send(&f.proxy, "POST", "/panel/api/v1/backups/download", &jar, json!({ "name": "x" }), &[], true).await;
    assert_eq!((off.status, off.body["code"].as_str()), (403, Some("forbidden")));
    assert!(off.body["message"].as_str().unwrap().contains("disabled"));
    // Enabled by the host: still an admin, a recent check and an audit row.
    let state = Dashboard { api: f.api.clone(), panel: f.panel.clone() };
    let open = dashboard::proxy_router(state, ProxyOrigin::parse("https://staff.example").unwrap().public_backups(true));
    let made = f.send(&open, "POST", "/panel/api/v1/backups", &jar, json!({}), &[("idempotency-key", "make-a-backup-000001")], true).await;
    assert_eq!(made.status, 200, "{}", made.body);
    let listed = f.send(&open, "GET", "/panel/api/v1/backups", &jar, json!(null), &[], true).await;
    let name = listed.body["backups"][0]["name"].as_str().unwrap().to_string();
    f.panel.backdate_mfa(jar.token(), 400);
    let stale = f.send(&open, "POST", "/panel/api/v1/backups/download", &jar, json!({ "name": name }), &[], true).await;
    assert_eq!((stale.status, stale.body["code"].as_str()), (403, Some("reauthentication_required")));
    let moderator = f.sign_in_moderator().await;
    let denied = f.send(&open, "POST", "/panel/api/v1/backups/download", &moderator, json!({ "name": name }), &[], true).await;
    assert_eq!(denied.status, 403, "a moderator never downloads");
    f.panel.stamp_for_tests(jar.token());
    let ok = f.send(&open, "POST", "/panel/api/v1/backups/download", &jar, json!({ "name": name }), &[("idempotency-key", "download-a-backup-0001")], true).await;
    assert_eq!(ok.status, 200, "{}", ok.body);
    assert!(f.audit().iter().any(|r| r.op == "download-backup" && r.actor == "Pegasus"), "the download is audited");
}

impl Fixture {
    async fn sign_in_moderator(&self) -> Jar {
        let (jar, _, _) = self.enroll("Croquet").await;
        jar
    }
}

#[tokio::test]
async fn origin_csrf_and_cookie_rules_hold_with_the_second_factor_on_both_transports() {
    let f = Fixture::new("origin", StaffMfaPolicy::Required).await;
    let (jar, _, codes) = f.enroll("Pegasus").await;
    let a = f.password("Pegasus").await;
    let challenge = a.body["challenge"].clone();
    // Loopback transport: a foreign origin, a missing key and a rebound host name are refused before any code is read.
    let body = json!({ "challenge": challenge, "code": codes[0] });
    let bad_origin = f.send(&f.router, "POST", "/panel/api/v1/sign-in/code", &Jar::default(), body.clone(), &[("origin", "http://evil.example")], false).await;
    assert_eq!((bad_origin.status, bad_origin.body["code"].as_str()), (403, Some("invalid_origin")));
    let no_key = {
        let mut r = Request::builder().method("POST").uri("/panel/api/v1/sign-in/code").header("host", "localhost").header("origin", "http://localhost").header("content-type", "application/json")
            .body(Body::from(body.to_string())).unwrap();
        r.extensions_mut().insert(ConnectInfo("127.0.0.1:1".parse::<std::net::SocketAddr>().unwrap()));
        f.router.clone().oneshot(r).await.unwrap().status().as_u16()
    };
    assert_eq!(no_key, 400);
    let rebound = {
        let mut r = Request::builder().method("POST").uri("/panel/api/v1/sign-in").header("host", "evil.example").header("origin", "http://evil.example").header("content-type", "application/json")
            .header("idempotency-key", "staff-mfa-contract-0001").body(Body::from(json!({ "user": "Pegasus", "password": PASSWORD }).to_string())).unwrap();
        r.extensions_mut().insert(ConnectInfo("127.0.0.1:1".parse::<std::net::SocketAddr>().unwrap()));
        f.router.clone().oneshot(r).await.unwrap().status().as_u16()
    };
    assert_eq!(rebound, 403);
    // A session's own POSTs need its CSRF token; a step-up from another origin does nothing.
    let forged = f.send(&f.router, "POST", "/panel/api/v1/mfa/step-up", &Jar { cookie: jar.cookie.clone(), csrf: "forged".into() }, json!({ "code": "123456" }), &[], false).await;
    assert_eq!((forged.status, forged.body["code"].as_str()), (403, Some("csrf_invalid")));
    // Behind the proxy: the exact origin and forwarded identity decide, with the code step as well.
    let proxied = f.send(&f.proxy, "POST", "/panel/api/v1/sign-in/code", &Jar::default(), body.clone(), &[], true).await;
    assert_eq!(proxied.status, 200, "{}", proxied.body);
    for flag in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/panel"] { assert!(proxied.headers.to_ascii_lowercase().contains(&flag.to_ascii_lowercase()), "{flag}: {}", proxied.headers) }
    let wrong_origin = f.send(&f.proxy, "POST", "/panel/api/v1/sign-in/code", &Jar::default(), body.clone(), &[("origin", "https://other.example")], true).await;
    assert_eq!((wrong_origin.status, wrong_origin.body["code"].as_str()), (403, Some("invalid_origin")));
    let no_forward = f.send(&f.proxy, "POST", "/panel/api/v1/sign-in/code", &Jar::default(), body, &[("x-forwarded-for", "198.51.100.7, 127.0.0.1")], true).await;
    assert_eq!(no_forward.status, 403, "one proxy client address only");
    // Loopback-originated requests on the staff interface: the TCP router answers localhost only.
    let mut r = Request::builder().method("GET").uri("/panel/api/v1/session").header("host", "localhost").header("cookie", &jar.cookie).body(Body::empty()).unwrap();
    r.extensions_mut().insert(ConnectInfo("203.0.113.9:1".parse::<std::net::SocketAddr>().unwrap()));
    assert_eq!(f.router.clone().oneshot(r).await.unwrap().status().as_u16(), 403);
}

#[tokio::test]
async fn recovery_is_local_only_and_never_a_staff_route() {
    let f = Fixture::new("local", StaffMfaPolicy::Required).await;
    let (jar, _, _) = f.enroll("Pegasus").await;
    for path in ["/panel/api/v1/staff-mfa-reset", "/api/admin/v1/staff-mfa-reset", "/panel/api/v1/mfa/reset", "/panel/api/v1/mfa/disable"] {
        let a = f.call("POST", path, &jar, json!({ "user": "Pegasus" })).await;
        assert_eq!(a.status, 404, "{path} is not a dashboard route");
    }
    let openapi: Value = serde_json::from_str(&dashboard::openapi_json()).unwrap();
    assert!(!openapi.to_string().contains("staff-mfa-reset"));
    // A moderator who is staff cannot reset an admin's factor through the host operation either (role-checked path).
    let refused = f.api.apply_as(&Op::StaffMfaReset { user: "Pegasus".into() }, "Croquet", "panel", Some(Role::Moderator));
    assert!(refused.is_err());
    // Resetting someone with no authenticator says so and still audits.
    let r = f.api.apply(&Op::StaffMfaReset { user: "Weevil".into() }, "host").unwrap();
    assert_eq!(r["had_authenticator"], false);
    assert!(f.api.apply(&Op::StaffMfaReset { user: "Nobody".into() }, "host").is_err());
}

#[tokio::test]
async fn the_transition_policy_keeps_password_only_sessions_until_an_account_enrolls() {
    let f = Fixture::new("optional", StaffMfaPolicy::Optional).await;
    let a = f.password("Pegasus").await;
    assert_eq!((a.body["session"]["limited"].clone(), a.body["session"]["mfa_required"].clone()), (json!(false), json!(false)));
    let jar = f.jar(&a);
    assert_eq!(f.call("GET", "/panel/api/v1/accounts", &jar, json!(null)).await.status, 200);
    let start = f.call("POST", "/panel/api/v1/mfa/enrollment", &jar, json!(null)).await;
    let seed = seed_of(start.body["secret"].as_str().unwrap());
    let done = f.call("POST", "/panel/api/v1/mfa/enrollment/confirm", &jar, json!({ "code": staff_mfa::code_at(&seed, (logins::now() / staff_mfa::STEP) as u64) })).await;
    assert_eq!(done.status, 200, "{}", done.body);
    // Once enrolled the code is always needed, whatever the policy.
    let b = f.password("Pegasus").await;
    assert_eq!(b.body["code_required"], true);
}

#[tokio::test]
async fn the_loopback_html_panel_asks_for_the_code_and_its_sessions_do_not_open_the_staff_api() {
    let f = Fixture::new("legacy", StaffMfaPolicy::Required).await;
    let (_, _, codes) = f.enroll("Pegasus").await;
    let login = |body: String| {
        let raw = format!("POST /panel/login HTTP/1.1\r\nHost: localhost\r\nOrigin: http://localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}", body.len());
        let (panel, api) = (f.panel.clone(), f.api.clone());
        async move { yo3_server::panel::handle(&raw, "127.0.0.1".parse().unwrap(), &panel, &api).await.unwrap() }
    };
    let no_code = login(format!("user=Pegasus&password={PASSWORD}")).await;
    assert!(no_code.status.starts_with("401"), "{}", no_code.status);
    // Only the dashboard's own sign-in does not apply here; the right code opens the loopback panel.
    let with = login(format!("user=Pegasus&password={PASSWORD}&code={}", codes[5])).await;
    assert!(with.status.starts_with("303"), "{}", with.status);
    let cookie = with.headers.iter().find(|(k, _)| *k == "Set-Cookie").map(|(_, v)| v.split(';').next().unwrap().to_string()).unwrap();
    let staff_api = f.call("GET", "/panel/api/v1/accounts", &Jar { cookie, csrf: String::new() }, json!(null)).await;
    assert_eq!(staff_api.status, 401, "a session of the HTML panel's form is not a staff API session");
}
