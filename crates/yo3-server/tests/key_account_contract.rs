//! Key creation and disclosure require a committed, secret-free audit/receipt boundary.
use std::{path::PathBuf, sync::{Arc, Mutex}};
use yo3_server::{accounts::Directory, api::{Api, Op}, logins::{Login, Registration}, settings, store};
struct Fixture { dir: PathBuf, api: Api, user: u32 }
impl Fixture {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("yo3-keys-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir); store::initialize(&dir).unwrap();
        let mut d = Directory::open(vec!["Yugi".into(), "Joey".into()], &dir.join("accounts.json")).unwrap();
        let user = d.register(Login::new("Owner", "PRIVATE_HASH".into(), "PRIVATE_RECOVERY".into()), None, Registration::Open).unwrap();
        let api = Api::new(Arc::new(Mutex::new(d)), Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(dir.clone()), Default::default(), "server");
        Self { dir, api, user }
    }
    fn sql(&self, sql: &str) { rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap(); }
}
impl Drop for Fixture { fn drop(&mut self) { store::close(&self.dir); let _ = std::fs::remove_dir_all(&self.dir); } }
#[test]
fn key_creation_and_disclosure_fail_closed_when_audit_cannot_be_saved() {
    for create in [true, false] {
        let f = Fixture::new(if create { "create-audit" } else { "reveal-audit" });
        f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'no audit'); END;");
        let op = if create { Op::CreateAccount { names: vec!["Kaiba".into()] } } else { Op::Key { who: "Owner".into() } };
        assert!(f.api.apply(&op, "host").is_err(), "{} reported success despite a failed audit", op.name());
        for d in [f.api.accounts.lock().unwrap().clone(), Directory::open(vec![], &f.dir.join("accounts.json")).unwrap()] {
            assert!(d.user_of("Kaiba").is_none()); assert!(d.entries().all(|(_, k, _)| k.is_none()));
        }
    }
}

use yo3_server::{api::{Actor, MutationErrorCode, keys::KeyAccountRequest as Request}, logins::Role};
const ID: &str = "key-account-test-0001";
fn req(f: &Fixture, create: bool) -> Request {
    if create { Request::Create { names: vec!["Kaiba".into()] } } else { Request::Reveal { user_id: f.user } }
}
#[test]
fn save_audit_receipt_and_commit_failures_leave_no_key_or_account_and_do_not_poison_later_saves() {
    for create in [true, false] { for failure in ["save", "audit", "receipt", "commit"] {
        let f = Fixture::new(&format!("{create}-{failure}"));
        match failure {
            "save" => store::fail_domain(&f.dir, "accounts.json", true).unwrap(),
            "audit" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'no audit'); END;"),
            "receipt" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL, 'no receipt'); END;"),
            "commit" => f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES(-1); END;"),
            _ => unreachable!(),
        }
        let request = req(&f, create);
        assert_eq!(f.api.key_account(&request, ID, Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::PersistenceFailed);
        for d in [f.api.accounts.lock().unwrap().clone(), Directory::open(vec![], &f.dir.join("accounts.json")).unwrap()] {
            assert!(d.user_of("Kaiba").is_none()); assert!(d.entries().all(|(_, key, _)| key.is_none()));
        }
        assert!(f.api.audit(100).unwrap().is_empty());
        if failure == "save" { store::fail_domain(&f.dir, "accounts.json", false).unwrap(); } else { f.sql("DROP TRIGGER refuse_change;"); }
        f.api.accounts.lock().unwrap().set_country_of(f.user, "FR").unwrap();
        assert!(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap().entries().all(|(_, key, _)| key.is_none()));
        let response = f.api.key_account(&request, ID, Actor::Host { by: "host" }, "server").unwrap();
        assert!(f.api.accounts.lock().unwrap().has_key(response.key.expose()));
        assert_eq!(response.user_id, if create { f.user + 1 } else { f.user });
        assert_eq!(f.api.audit(100).unwrap().len(), 1);
    } }
}
#[test]
fn concurrent_retries_restart_and_pruned_audit_never_duplicate_accounts_or_store_credentials() {
    let f = Fixture::new("retry"); let request = req(&f, true);
    let responses = std::thread::scope(|s| {
        let jobs: Vec<_> = (0..8).map(|_| s.spawn(|| f.api.key_account(&request, ID, Actor::Host { by: "host" }, "server").unwrap())).collect();
        jobs.into_iter().map(|job| job.join().unwrap()).collect::<Vec<_>>()
    });
    assert!(responses.iter().all(|r| r == &responses[0])); assert_eq!(f.api.audit(100).unwrap().len(), 1);
    let secret = responses[0].key.expose();
    assert!(!format!("{:?}", responses[0]).contains(secret));
    f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';"); store::prune_audit(&f.dir, 1).unwrap();
    store::close(&f.dir); store::initialize(&f.dir).unwrap();
    let api = Api::new(Arc::new(Mutex::new(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap())), Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(f.dir.clone()), Default::default(), "offline");
    assert_eq!(api.key_account(&request, ID, Actor::Host { by: "another host label" }, "offline").unwrap(), responses[0]);
    let saved: String = rusqlite::Connection::open(f.dir.join("server.db")).unwrap().query_row("SELECT result FROM operation_receipts", [], |row| row.get(0)).unwrap();
    for value in [secret, "Kaiba", "host", "PRIVATE"] { assert!(!saved.contains(value), "receipt leaked {value}"); }
    assert_eq!(api.key_account(&Request::Create { names: vec!["Other".into()] }, ID, Actor::Host { by: "host" }, "offline").unwrap_err().code, MutationErrorCode::RequestConflict);
    api.accounts.lock().unwrap().delete_account(responses[0].user_id).unwrap();
    assert_eq!(api.key_account(&request, ID, Actor::Host { by: "host" }, "offline").unwrap_err().code, MutationErrorCode::NotFound);
    assert!(api.accounts.lock().unwrap().user_of("Kaiba").is_none());
}
#[test]
fn existing_keys_are_stable_and_disclosure_requires_current_admin_authority_even_on_retry() {
    let f = Fixture::new("permissions");
    let admin = f.api.accounts.lock().unwrap().register(Login::new("Admin", "HASH".into(), "RECOVERY".into()), None, Registration::Open).unwrap();
    f.api.accounts.lock().unwrap().set_role("Admin", Role::Admin).unwrap();
    let request = req(&f, false);
    let result = f.api.key_account(&request, ID, Actor::Staff { user_id: admin }, "panel").unwrap();
    let again = f.api.key_account(&request, "new-key-reveal-00001", Actor::Host { by: "host" }, "server").unwrap();
    assert_eq!(result.key, again.key);
    f.api.accounts.lock().unwrap().set_role("Admin", Role::Moderator).unwrap();
    assert_eq!(f.api.key_account(&request, ID, Actor::Staff { user_id: admin }, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    let rows = serde_json::to_string(&f.api.audit(100).unwrap()).unwrap();
    assert!(!rows.contains(result.key.expose())); assert_eq!(f.api.audit(100).unwrap().len(), 2);
}
#[test]
fn names_are_validated_and_independent_requests_preserve_both_accounts() {
    let f = Fixture::new("names");
    for names in [vec![" ".into()], vec!["One".into(), "one".into()], vec!["x".into();5], vec!["x".repeat(17)], vec!["x\ny".into()]] {
        assert_eq!(f.api.key_account(&Request::Create { names }, ID, Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
    }
    std::thread::scope(|s| { for (name, id) in [("NewOne", "independent-keys-0001"), ("NewTwo", "independent-keys-0002")] {
        let api = &f.api; s.spawn(move || api.key_account(&Request::Create { names: vec![name.into()] }, id, Actor::Host { by: "host" }, "server").unwrap());
    } });
    assert!(f.api.accounts.lock().unwrap().user_of("NewOne").is_some()); assert!(f.api.accounts.lock().unwrap().user_of("NewTwo").is_some());
    let result = f.api.key_account(&Request::Create { names: vec![] }, ID, Actor::Host { by: "host" }, "server").unwrap();
    assert_eq!(result.players.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["Yugi", "Joey"]);
}
#[test]
fn cli_retries_use_the_same_host_receipt_for_creation_key_and_profile_export() {
    let f = Fixture::new("cli");
    let run = |args: &[&str]| {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-server")).args(["--data-dir", f.dir.to_str().unwrap()]).args(args).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr)); String::from_utf8(out.stdout).unwrap()
    };
    let first = run(&["--by", "One", "add-account", "Kaiba", "--request-id", ID]);
    assert_eq!(run(&["--by", "Two", "add-account", "Kaiba", "--request-id", ID]), first);
    let key = run(&["key", "Kaiba", "--request-id", "cli-key-reveal-00001"]);
    assert_eq!(key, first);
    let file = f.dir.join("private-profile.json");
    run(&["profile", "Kaiba", file.to_str().unwrap(), "--request-id", "cli-profile-test-0001"]);
    let before = std::fs::read(&file).unwrap();
    std::fs::remove_file(&file).unwrap();
    run(&["profile", "Kaiba", file.to_str().unwrap(), "--request-id", "cli-profile-test-0001"]);
    assert_eq!(std::fs::read(&file).unwrap(), before);
    assert_eq!(f.api.audit(100).unwrap().len(), 3);
}
