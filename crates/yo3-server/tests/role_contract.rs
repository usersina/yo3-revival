//! Staff roles: committed state, audit and receipt must agree, including after retries.
use std::{path::PathBuf, sync::{Arc, Mutex}};
use yo3_server::{accounts::Directory, api::{Api, Op}, logins::{self, Login, Registration, Role}, settings, store};

struct Fixture { dir: PathBuf, api: Api, user: u32 }
impl Fixture {
    fn new(name: &str) -> Self {
        logins::use_fast_hashing();
        let dir = std::env::temp_dir().join(format!("yo3-role-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        store::initialize(&dir).unwrap();
        let mut d = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
        let hash = logins::hash_password_now("LongPassword123").unwrap();
        let user = d.register(Login::new("Yugi", hash.clone(), "PRIVATE_RECOVERY".into()), None, Registration::Open).unwrap();
        d.register(Login::new("Pegasus", hash.clone(), "PRIVATE_ADMIN_RECOVERY".into()), None, Registration::Open).unwrap();
        d.set_role("Pegasus", Role::Admin).unwrap();
        d.register(Login::new("Croquet", hash, "PRIVATE_MOD_RECOVERY".into()), None, Registration::Open).unwrap();
        d.set_role("Croquet", Role::Moderator).unwrap();
        let api = Api::new(Arc::new(Mutex::new(d)), Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(dir.clone()), Default::default(), "server");
        Self { dir, api, user }
    }
    fn sql(&self, sql: &str) { rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap(); }
    fn role(&self, disk: bool) -> Role {
        if disk { Directory::open(vec![], &self.dir.join("accounts.json")).unwrap().login_by_id(self.user).unwrap().role }
        else { self.api.accounts.lock().unwrap().login_by_id(self.user).unwrap().role }
    }
}
impl Drop for Fixture { fn drop(&mut self) { store::close(&self.dir); let _ = std::fs::remove_dir_all(&self.dir); } }
const ID: &str = "role-change-000000001";
fn op() -> Op { Op::SetRole { user: "Yugi".into(), role: "moderator".into() } }

#[test]
fn a_role_change_cannot_report_success_without_its_audit() {
    let f = Fixture::new("audit");
    f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'no audit'); END;");
    assert!(f.api.apply_request(&op(), "Anis", "server", None, ID).is_err(), "role change reported success without audit");
    assert_eq!(f.role(false), Role::Player); assert_eq!(f.role(true), Role::Player);
}

use yo3_server::api::{Actor, MutationErrorCode, profiles::AccountEditRequest};
fn request(f: &Fixture, role: Role) -> AccountEditRequest { AccountEditRequest::SetRole { user_id: f.user, role } }

#[test]
fn role_save_audit_receipt_and_commit_failures_leave_no_partial_state_or_epoch() {
    for failure in ["save", "audit", "receipt", "commit"] {
        let f = Fixture::new(failure);
        let before = f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().clone();
        match failure {
            "save" => store::fail_domain(&f.dir, "accounts.json", true).unwrap(),
            "audit" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'no audit'); END;"),
            "receipt" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL, 'no receipt'); END;"),
            "commit" => f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
                CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES (-1); END;"),
            _ => unreachable!(),
        }
        assert_eq!(f.api.edit_account(&request(&f, Role::Moderator), ID, Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::PersistenceFailed);
        assert_eq!(f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap(), &before);
        assert_eq!(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap().login_by_id(f.user).unwrap(), &before);
        assert!(f.api.audit(100).unwrap().is_empty());
        if failure == "save" { store::fail_domain(&f.dir, "accounts.json", false).unwrap(); } else { f.sql("DROP TRIGGER refuse_change;"); }
        f.api.accounts.lock().unwrap().set_country_of(f.user, "FR").unwrap();
        assert_eq!(f.role(true), Role::Player, "a subsequent save cannot leak a rejected role");
        let result = f.api.edit_account(&request(&f, Role::Moderator), ID, Actor::Host { by: "host" }, "server").unwrap();
        assert_eq!(result.user_id, f.user); assert_eq!(f.role(true), Role::Moderator);
        assert_eq!(f.api.audit(100).unwrap().len(), 1);
        let epoch = f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().staff_epoch;
        assert_eq!(epoch, before.staff_epoch + 1);
    }
}

#[test]
fn concurrent_role_retries_commit_once_and_later_roles_are_not_reversed_by_replay() {
    let f = Fixture::new("concurrent");
    let req = request(&f, Role::Moderator);
    let results = std::thread::scope(|s| {
        let jobs: Vec<_> = (0..8).map(|_| s.spawn(|| f.api.edit_account(&req, ID, Actor::Host { by: "Anis" }, "server").unwrap())).collect();
        jobs.into_iter().map(|job| job.join().unwrap()).collect::<Vec<_>>()
    });
    assert!(results.iter().all(|result| result == &results[0]));
    assert_eq!(f.api.audit(100).unwrap().len(), 1);
    assert_eq!(f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().staff_epoch, 1);
    f.api.edit_account(&request(&f, Role::Admin), "independent-role-00001", Actor::Host { by: "host" }, "server").unwrap();
    f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';"); store::prune_audit(&f.dir, 1).unwrap();
    store::close(&f.dir); store::initialize(&f.dir).unwrap();
    let api = Api::new(Arc::new(Mutex::new(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap())), Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(f.dir.clone()), Default::default(), "offline");
    assert_eq!(api.edit_account(&req, ID, Actor::Host { by: "another audit label" }, "offline").unwrap(), results[0]);
    assert_eq!(api.accounts.lock().unwrap().login_by_id(f.user).unwrap().role, Role::Admin);
    assert_eq!(api.accounts.lock().unwrap().login_by_id(f.user).unwrap().staff_epoch, 2);
    assert_eq!(api.edit_account(&request(&f, Role::Player), ID, Actor::Host { by: "host" }, "offline").unwrap_err().code, MutationErrorCode::RequestConflict);
    assert!(api.audit(100).unwrap().is_empty());
    let saved: String = rusqlite::Connection::open(f.dir.join("server.db")).unwrap().query_row("SELECT result FROM operation_receipts LIMIT 1", [], |r| r.get(0)).unwrap();
    for secret in ["PRIVATE", "Yugi", "Anis", "password", "recovery", "staff_epoch"] { assert!(!saved.contains(secret), "receipt exposed {secret}"); }
}

#[test]
fn current_admin_authority_is_required_for_role_changes_including_retries_and_self_changes() {
    let f = Fixture::new("permissions");
    let (admin, moderator) = { let d = f.api.accounts.lock().unwrap(); (d.login_of("Pegasus").unwrap().0, d.login_of("Croquet").unwrap().0) };
    let req = request(&f, Role::Moderator);
    assert_eq!(f.api.edit_account(&req, ID, Actor::Staff { user_id: moderator }, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    let first = f.api.edit_account(&req, ID, Actor::Staff { user_id: admin }, "panel").unwrap();
    f.api.accounts.lock().unwrap().set_role("Pegasus", Role::Moderator).unwrap();
    assert_eq!(f.api.edit_account(&req, ID, Actor::Staff { user_id: admin }, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    f.api.accounts.lock().unwrap().set_role("Pegasus", Role::Admin).unwrap();
    assert_eq!(f.api.edit_account(&req, ID, Actor::Staff { user_id: admin }, "panel").unwrap(), first);
    let self_edit = AccountEditRequest::SetRole { user_id: admin, role: Role::Player };
    f.api.edit_account(&self_edit, "self-demotion-0000001", Actor::Staff { user_id: admin }, "panel").unwrap();
    assert_eq!(f.api.edit_account(&self_edit, "self-demotion-0000001", Actor::Staff { user_id: admin }, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    f.api.accounts.lock().unwrap().add_account(&["Kaiba".into()]).unwrap();
    let key_user = f.api.accounts.lock().unwrap().user_of("Kaiba").unwrap();
    assert_eq!(f.api.edit_account(&AccountEditRequest::SetRole { user_id: key_user, role: Role::Admin }, "key-role-00000000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
    assert_eq!(f.api.edit_account(&AccountEditRequest::SetRole { user_id: u32::MAX, role: Role::Admin }, "missing-role-00000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::NotFound);
}

#[test]
fn old_login_rows_default_the_epoch_and_noop_roles_do_not_invalidate_new_sessions() {
    let f = Fixture::new("epoch");
    let login = f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().clone();
    let mut value = serde_json::to_value(&login).unwrap(); value.as_object_mut().unwrap().remove("staff_epoch");
    assert_eq!(serde_json::from_value::<Login>(value).unwrap().staff_epoch, 0);
    f.api.edit_account(&request(&f, Role::Player), ID, Actor::Host { by: "host" }, "server").unwrap();
    assert_eq!(f.api.accounts.lock().unwrap().login_by_id(f.user).unwrap().staff_epoch, 0);
}

#[test]
fn cli_role_retries_use_the_fixed_host_actor_and_do_not_reapply_a_later_change() {
    let f = Fixture::new("cli");
    let run = |role: &str, by: &str| {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-server"))
            .args(["--data-dir", f.dir.to_str().unwrap(), "--by", by, "set-role", "Yugi", role, "--request-id", "cli-role-change-00001"]).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    };
    let first = run("moderator", "Anis");
    f.api.accounts.lock().unwrap().set_role("Yugi", Role::Admin).unwrap();
    assert_eq!(run("moderator", "Another audit label"), first);
    assert_eq!(f.role(true), Role::Admin);
    assert_eq!(f.api.audit(100).unwrap().iter().filter(|r| r.op == "set-role").count(), 1);
}

#[test]
fn independent_concurrent_role_changes_preserve_both_accounts_and_audit_entries() {
    let f = Fixture::new("independent");
    let moderator = f.api.accounts.lock().unwrap().login_of("Croquet").unwrap().0;
    std::thread::scope(|s| {
        for (user_id, role, id) in [(f.user, Role::Moderator, "independent-role-0001"), (moderator, Role::Admin, "independent-role-0002")] {
            let api = &f.api;
            s.spawn(move || api.edit_account(&AccountEditRequest::SetRole { user_id, role }, id, Actor::Host { by: "host" }, "server").unwrap());
        }
    });
    let saved = Directory::open(vec![], &f.dir.join("accounts.json")).unwrap();
    assert_eq!(saved.login_by_id(f.user).unwrap().role, Role::Moderator);
    assert_eq!(saved.login_by_id(moderator).unwrap().role, Role::Admin);
    let rows = f.api.audit(100).unwrap(); assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.op == "set-role" && row.before.is_some() && row.after.is_some()));
}
