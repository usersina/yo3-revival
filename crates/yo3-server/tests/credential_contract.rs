//! Password-account provisioning and reset issuance must commit with their audit and retry receipt.
use std::{path::PathBuf, sync::{Arc, Mutex}};
use yo3_server::{accounts::Directory, api::{Api, Op}, logins::{self, Login, Registration, Role}, settings, store};
struct Fixture { dir: PathBuf, api: Api, user: u32 }
impl Fixture {
    fn new(name: &str) -> Self {
        logins::use_fast_hashing();
        let dir = std::env::temp_dir().join(format!("yo3-credentials-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir); store::initialize(&dir).unwrap();
        let mut d = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
        let hash = logins::hash_password_now("LongPassword123").unwrap();
        let user = d.register(Login::new("Yugi", hash.clone(), "PRIVATE_RECOVERY".into()), None, Registration::Open).unwrap();
        for (user, role) in [("Pegasus", Role::Admin), ("Croquet", Role::Moderator)] {
            d.register(Login::new(user, hash.clone(), "PRIVATE_RECOVERY".into()), None, Registration::Open).unwrap(); d.set_role(user, role).unwrap();
        }
        let api = Api::new(Arc::new(Mutex::new(d)), Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(dir.clone()), Default::default(), "server");
        Self { dir, api, user }
    }
    fn sql(&self, sql: &str) { rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap(); }
}
impl Drop for Fixture { fn drop(&mut self) { store::close(&self.dir); let _ = std::fs::remove_dir_all(&self.dir); } }
#[test]
fn credential_changes_cannot_report_success_when_their_audit_fails() {
    for create in [true, false] {
        let f = Fixture::new(if create { "create-audit" } else { "reset-audit" });
        let op = if create { Op::AddUser { user: "Joey".into(), code: logins::new_code().into() } }
            else { Op::ResetCode { user: "Yugi".into(), code: logins::new_code().into() } };
        f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'no audit'); END;");
        assert!(f.api.apply(&op, "host").is_err(), "{} reported success without its audit", op.name());
        let d = Directory::open(vec![], &f.dir.join("accounts.json")).unwrap();
        assert!(d.login_of("Joey").is_none()); assert!(d.login_by_id(f.user).unwrap().reset.is_none());
    }
}

use yo3_server::api::{Actor, MutationErrorCode, credentials::{IssueAccountCodeRequest as Request, AccountCodeState}};
const CODE: &str = "ABCD-EFGH-JKLM-NPQR";
const OTHER: &str = "QRST-UVWX-YZ23-4567";
const ID: &str = "credential-test-00001";
fn request(f: &Fixture, create: bool, code: &str) -> Request {
    if create { Request::CreateAccount { user: "Joey".into(), code: code.to_string().into() } }
    else { Request::ResetPassword { user_id: f.user, code: code.to_string().into() } }
}
#[test]
fn account_reset_audit_receipt_and_commit_failures_roll_back_credentials_and_subsequent_saves() {
    for create in [true, false] {
        for failure in ["save", "audit", "receipt", "commit"] {
            let f = Fixture::new(&format!("{create}-{failure}"));
            let before = f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().clone();
            match failure {
                "save" => store::fail_domain(&f.dir, "accounts.json", true).unwrap(),
                "audit" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'no audit'); END;"),
                "receipt" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL, 'no receipt'); END;"),
                "commit" => f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES(-1); END;"),
                _ => unreachable!(),
            }
            let req = request(&f, create, CODE);
            assert_eq!(f.api.issue_account_code(&req, ID, Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::PersistenceFailed);
            for d in [f.api.accounts.lock().unwrap().clone(), Directory::open(vec![], &f.dir.join("accounts.json")).unwrap()] {
                assert!(d.login_of("Joey").is_none()); assert_eq!(d.login_by_id(f.user).unwrap(), &before);
            }
            assert!(f.api.audit(100).unwrap().is_empty());
            if failure == "save" { store::fail_domain(&f.dir, "accounts.json", false).unwrap(); } else { f.sql("DROP TRIGGER refuse_change;"); }
            f.api.accounts.lock().unwrap().set_country_of(f.user, "FR").unwrap();
            assert!(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap().login_of("Joey").is_none());
            assert_eq!(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap().login_by_id(f.user).unwrap(), &before);
            let issued = f.api.issue_account_code(&req, ID, Actor::Host { by: "host" }, "server").unwrap();
            assert_eq!(issued.state, AccountCodeState::Ready);
            assert_eq!(f.api.audit(100).unwrap().len(), 1);
            if create { assert_eq!(issued.confirmation.user_id, f.user + 3, "failure must not consume an account ID"); }
            let db = rusqlite::Connection::open(f.dir.join("server.db")).unwrap();
            let stored: String = db.query_row("SELECT result || input FROM operation_receipts", [], |r| r.get(0)).unwrap();
            for private in [CODE, "ABCDEFGHIJKLMNOPQR", "Joey", "PRIVATE_RECOVERY", "password"] { assert!(!stored.contains(private), "receipt retained {private}"); }
            assert!(!serde_json::to_string(&f.api.audit(100).unwrap()).unwrap().contains(CODE));
        }
    }
}
#[test]
fn duplicate_creation_restarts_and_audit_pruning_keep_only_one_account_without_reactivating_codes() {
    let f = Fixture::new("retry"); let req = request(&f, true, CODE);
    let results = std::thread::scope(|s| { let jobs: Vec<_> = (0..8).map(|_| s.spawn(|| f.api.issue_account_code(&req, ID, Actor::Host { by: "Anis" }, "server").unwrap())).collect(); jobs.into_iter().map(|j| j.join().unwrap()).collect::<Vec<_>>() });
    assert!(results.iter().all(|r| r == &results[0])); assert_eq!(f.api.audit(100).unwrap().len(), 1);
    let user = results[0].confirmation.user_id;
    assert!(f.api.accounts.lock().unwrap().login_by_id(user).unwrap().password.is_empty());
    let hash = logins::hash_password_now("ChosenPassword1").unwrap();
    f.api.accounts.lock().unwrap().set_password(user, hash.clone()).unwrap();
    f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';"); store::prune_audit(&f.dir, 1).unwrap(); store::close(&f.dir); store::initialize(&f.dir).unwrap();
    let api = Api::new(Arc::new(Mutex::new(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap())), Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(f.dir.clone()), Default::default(), "offline");
    let replay = api.issue_account_code(&req, ID, Actor::Host { by: "another audit label" }, "offline").unwrap();
    assert_eq!(replay.confirmation, results[0].confirmation); assert_eq!(replay.state, AccountCodeState::UsedOrReplaced);
    assert_eq!(api.accounts.lock().unwrap().login_by_id(user).unwrap().password, hash);
    assert_eq!(api.accounts.lock().unwrap().entries().count(), 4); assert!(api.audit(100).unwrap().is_empty());
    assert_eq!(api.issue_account_code(&request(&f, true, OTHER), ID, Actor::Host { by: "host" }, "offline").unwrap_err().code, MutationErrorCode::RequestConflict);
    assert_eq!(api.issue_account_code(&req, "other-create-id-00001", Actor::Host { by: "host" }, "offline").unwrap_err().code, MutationErrorCode::AlreadyExists);
}
#[test]
fn reset_retries_do_not_replace_a_later_code_and_permissions_are_rechecked_before_replay() {
    let f = Fixture::new("permissions");
    let (admin, moderator) = { let d = f.api.accounts.lock().unwrap(); (d.login_of("Pegasus").unwrap().0, d.login_of("Croquet").unwrap().0) };
    let actor = Actor::Staff { user_id: moderator };
    let req = request(&f, false, CODE);
    assert_eq!(f.api.issue_account_code(&request(&f, true, CODE), ID, actor, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    let first = f.api.issue_account_code(&req, ID, actor, "panel").unwrap();
    f.api.issue_account_code(&request(&f, false, OTHER), "replacement-code-0001", Actor::Host { by: "host" }, "server").unwrap();
    let retry = f.api.issue_account_code(&req, ID, actor, "panel").unwrap();
    assert_eq!(first.confirmation, retry.confirmation); assert_eq!(retry.state, AccountCodeState::UsedOrReplaced);
    f.api.accounts.lock().unwrap().set_role("Yugi", Role::Admin).unwrap();
    assert!(f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().reset.is_none(), "a code issued before promotion must lose its authority");
    assert_eq!(f.api.issue_account_code(&req, ID, actor, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    assert_eq!(f.api.issue_account_code(&Request::ResetPassword { user_id: admin, code: CODE.to_string().into() }, "reset-admin-00000001", actor, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    f.api.issue_account_code(&Request::ResetPassword { user_id: moderator, code: CODE.to_string().into() }, "reset-own-0000000001", actor, "panel").unwrap();
    f.api.accounts.lock().unwrap().set_role("Croquet", Role::Player).unwrap();
    assert_eq!(f.api.issue_account_code(&req, ID, actor, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
}
#[test]
fn recovery_is_one_use_atomic_and_rechecks_code_replacement_expiry_and_role_revision() {
    for failure in ["save", "replaced", "expired", "role", "success"] {
        let f = Fixture::new(&format!("recover-{failure}"));
        let issued = f.api.issue_account_code(&request(&f, failure != "role", CODE), ID, Actor::Host { by: "host" }, "server").unwrap();
        let user = issued.confirmation.user_id;
        let before = f.api.accounts.lock().unwrap().login_by_id(user).unwrap().clone();
        let hash = logins::hash_password_now("ChosenPassword1").unwrap();
        if failure == "save" { store::fail_domain(&f.dir, "accounts.json", true).unwrap(); }
        if failure == "replaced" { f.api.accounts.lock().unwrap().issue_reset("joey", logins::digest(&logins::normalize_code(OTHER)), logins::now() + 86400).unwrap(); }
        if failure == "role" { f.api.accounts.lock().unwrap().set_role("Yugi", Role::Admin).unwrap(); }
        let now = if failure == "expired" { issued.confirmation.expires_at.timestamp() } else { logins::now() };
        let result = f.api.accounts.lock().unwrap().complete_recovery(user, &before, &logins::digest(&logins::normalize_code(CODE)), hash.clone(), "NEW_PRIVATE_RECOVERY_DIGEST".into(), now);
        if failure != "success" {
            assert!(result.is_err()); assert_eq!(f.api.accounts.lock().unwrap().login_by_id(user).unwrap().password, before.password);
            assert_eq!(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap().login_by_id(user).unwrap().password, before.password);
            if failure != "save" { continue } store::fail_domain(&f.dir, "accounts.json", false).unwrap();
        } else { assert!(result.unwrap()); }
        let mut d = f.api.accounts.lock().unwrap();
        if failure == "save" { assert!(d.complete_recovery(user, &before, &logins::digest(&logins::normalize_code(CODE)), hash.clone(), "NEW_PRIVATE_RECOVERY_DIGEST".into(), now).unwrap()); }
        assert!(d.complete_recovery(user, &before, &logins::digest(&logins::normalize_code(CODE)), hash, "ANOTHER_RECOVERY".into(), now).is_err());
        let login = d.login_by_id(user).unwrap(); assert!(login.reset.is_none()); assert_eq!(login.recovery, "NEW_PRIVATE_RECOVERY_DIGEST");
    }
}
#[test]
fn codes_are_validated_canonicalized_and_never_disclosed_by_debug_formatting() {
    let f = Fixture::new("validation");
    let req = request(&f, false, CODE); let first = f.api.issue_account_code(&req, ID, Actor::Host { by: "host" }, "server").unwrap();
    assert!(!format!("{req:?}").contains(CODE));
    assert_eq!(first, f.api.issue_account_code(&request(&f, false, "abcdefghjklmnpqr"), ID, Actor::Host { by: "host" }, "server").unwrap());
    for code in ["", "short", "ABCD--EFGHJKLMNPQR", "ABCD EFGH JKLM NPQR", "12345678901234567890"] {
        assert_eq!(f.api.issue_account_code(&request(&f, false, code), "invalid-code-00000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
    }
    for user in ["", "ab", "bad name", "12345678901234567"] {
        assert_eq!(f.api.issue_account_code(&Request::CreateAccount { user: user.into(), code: CODE.to_string().into() }, "invalid-user-00000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
    }
}

#[test]
fn cli_codes_are_prepared_before_mutation_and_retried_from_stdin_without_exposing_arguments() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let f = Fixture::new("cli");
    let run = |operation: &str, user: &str, code: &str, id: &str| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_yo3-server")).args(["--data-dir", f.dir.to_str().unwrap(), operation, user, "--request-id", id, "--code-stdin"])
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(code.as_bytes()).unwrap(); let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr)); String::from_utf8(output.stdout).unwrap()
    };
    let first = run("add-user", "Joey", CODE, "cli-create-user-00001"); assert!(first.contains("Prepared code") && first.contains("ready"));
    assert_eq!(run("add-user", "Joey", CODE, "cli-create-user-00001"), first);
    let reset = run("reset-code", "Joey", OTHER, "cli-reset-user-000001");
    assert_eq!(run("reset-code", "Joey", OTHER, "cli-reset-user-000001"), reset);
    assert_eq!(f.api.audit(100).unwrap().len(), 2);
}

#[test]
fn a_new_owner_must_complete_setup_before_receiving_a_staff_role() {
    use yo3_server::api::profiles::AccountEditRequest;
    let f = Fixture::new("staff-setup");
    let issued = f.api.issue_account_code(&request(&f, true, CODE), ID, Actor::Host { by: "host" }, "server").unwrap();
    let user = issued.confirmation.user_id;
    let role = AccountEditRequest::SetRole { user_id: user, role: Role::Admin };
    assert_eq!(f.api.edit_account(&role, "unclaimed-admin-00001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
    let before = f.api.accounts.lock().unwrap().login_by_id(user).unwrap().clone();
    f.api.accounts.lock().unwrap().complete_recovery(user, &before, &logins::digest(&logins::normalize_code(CODE)), logins::hash_password_now("ChosenPassword1").unwrap(), "OWNER_RECOVERY".into(), logins::now()).unwrap();
    f.api.edit_account(&role, "unclaimed-admin-00001", Actor::Host { by: "host" }, "server").unwrap();
}

#[test]
fn a_password_change_cannot_commit_after_its_old_password_or_authority_was_replaced() {
    let f = Fixture::new("stale-password");
    let before = f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().clone();
    let newer = logins::hash_password_now("NewerPassword12").unwrap();
    let stolen = logins::hash_password_now("StolenPassword1").unwrap();
    f.api.accounts.lock().unwrap().set_password(f.user, newer.clone()).unwrap();
    assert!(f.api.accounts.lock().unwrap().change_password_if_current(f.user, &before, stolen).is_err());
    assert_eq!(f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().password, newer);
}
