//! Invite administration must commit the invite, audit and retry receipt before publication.
use std::{path::PathBuf, sync::{Arc, Mutex}};
use yo3_server::{accounts::Directory, api::{Api, Op, Actor, MutationErrorCode, invites::{CreateInviteRequest, InviteState}}, settings, store};

struct Fixture { dir: PathBuf, api: Api }
impl Fixture {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("yo3-invite-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        store::initialize(&dir).unwrap();
        let accounts = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
        let api = Api::new(Arc::new(Mutex::new(accounts)), Arc::new(settings::Live::open(None, vec![]).unwrap()),
            Some(dir.clone()), Default::default(), "server");
        Self { dir, api }
    }
    fn sql(&self, sql: &str) { rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap(); }
    fn counts(&self) -> (usize, usize) {
        (self.api.accounts.lock().unwrap().invites().len(), Directory::open(vec![], &self.dir.join("accounts.json")).unwrap().invites().len())
    }
}
impl Drop for Fixture { fn drop(&mut self) { store::close(&self.dir); let _ = std::fs::remove_dir_all(&self.dir); } }

#[test]
fn invites_cannot_succeed_without_their_audit() {
    let f = Fixture::new("audit");
    f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'audit unavailable'); END;");
    let result = f.api.apply(&Op::Invite { uses: Some(2), days: Some(7) }, "Anis");
    assert!(result.is_err(), "invite without audit reported success: {result:?}");
    assert_eq!(f.counts(), (0, 0));
}

#[test]
fn retrying_an_invite_returns_the_original_code_and_expiry() {
    let f = Fixture::new("retry");
    let op = Op::Invite { uses: Some(2), days: Some(7) };
    let first = f.api.apply_request(&op, "Anis", "server", None, "invite-retry-00000001").unwrap();
    let retry = f.api.apply_request(&op, "new audit name", "offline", None, "invite-retry-00000001").unwrap();
    assert_eq!(retry, first);
    assert_eq!(f.counts(), (1, 1));
    assert_eq!(f.api.audit(100).unwrap().len(), 1);
}

const ID: &str = "invite-create-00000001";
fn request() -> CreateInviteRequest { CreateInviteRequest { uses: Some(2), days: Some(7) } }

#[test]
fn save_receipt_and_commit_failures_publish_nothing_and_a_later_save_stays_correct() {
    for failure in ["save", "receipt", "commit"] {
        let f = Fixture::new(failure);
        match failure {
            "save" => store::fail_domain(&f.dir, "accounts.json", true).unwrap(),
            "receipt" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL, 'no receipt'); END;"),
            "commit" => f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY);
                CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
                CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES (-1); END;"),
            _ => unreachable!(),
        }
        assert_eq!(f.api.create_invite(&request(), ID, Actor::Host { by: "Anis" }, "server").unwrap_err().code, MutationErrorCode::PersistenceFailed);
        assert_eq!(f.counts(), (0, 0));
        assert!(f.api.audit(100).unwrap().is_empty());
        assert_eq!(rusqlite::Connection::open(f.dir.join("server.db")).unwrap().query_row("SELECT count(*) FROM operation_receipts", [], |r| r.get::<_, u32>(0)).unwrap(), 0);
        if failure == "save" { store::fail_domain(&f.dir, "accounts.json", false).unwrap(); }
        else { f.sql("DROP TRIGGER refuse_change;"); }
        f.api.accounts.lock().unwrap().add_account(&["Yugi".into()]).unwrap();
        assert_eq!(f.counts(), (0, 0));
        f.api.create_invite(&request(), ID, Actor::Host { by: "Anis" }, "server").unwrap();
        assert_eq!(f.counts(), (1, 1));
        assert_eq!(f.api.audit(100).unwrap().len(), 1);
    }
}

#[test]
fn concurrent_duplicate_and_independent_invites_use_distinct_receipts() {
    let f = Fixture::new("concurrent");
    let request = request();
    let results = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8).map(|_| s.spawn(|| f.api.create_invite(&request, ID, Actor::Host { by: "Anis" }, "server").unwrap())).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()
    });
    assert!(results.iter().all(|r| r == &results[0]));
    assert_eq!(f.counts(), (1, 1));
    std::thread::scope(|s| {
        for id in ["invite-independent-00001", "invite-independent-00002"] {
            s.spawn(|| f.api.create_invite(&request, id, Actor::Host { by: "Anis" }, "server").unwrap());
        }
    });
    assert_eq!(f.counts(), (3, 3));
    assert_eq!(f.api.audit(100).unwrap().len(), 3);
    assert_eq!(f.api.create_invite(&CreateInviteRequest { uses: Some(1), ..request.clone() }, ID, Actor::Host { by: "Anis" }, "server")
        .unwrap_err().code, MutationErrorCode::RequestConflict);
    assert!(f.api.apply_request(&Op::AddCode { code: String::new(), bp: 100, uses: None }, "Anis", "server", None, ID).is_err());
}

#[test]
fn retries_after_consumption_restart_and_audit_pruning_keep_the_original_result_and_do_not_replenish() {
    use yo3_server::logins::{self, Login, Registration};
    logins::use_fast_hashing();
    let f = Fixture::new("restart");
    let request = CreateInviteRequest { uses: Some(1), days: Some(7) };
    let first = f.api.create_invite(&request, ID, Actor::Host { by: "Anis" }, "server").unwrap();
    let user = f.api.accounts.lock().unwrap().register(Login::new("Yugi", logins::hash_password_now("LongPassword123").unwrap(), "recovery".into()),
        Some(&first.code), Registration::Invite).unwrap();
    assert_eq!(f.api.list_invites(Actor::Host { by: "Anis" }).unwrap().invites[0].state, InviteState::Exhausted);
    f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';");
    store::prune_audit(&f.dir, 1).unwrap();
    store::close(&f.dir); store::initialize(&f.dir).unwrap();
    let api = Api::new(Arc::new(Mutex::new(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap())),
        Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(f.dir.clone()), Default::default(), "offline");
    assert_eq!(api.create_invite(&request, ID, Actor::Host { by: "different audit name" }, "offline").unwrap(), first);
    let list = api.list_invites(Actor::Host { by: "Anis" }).unwrap();
    assert_eq!(list.invites.len(), 1);
    assert_eq!(list.invites[0].uses, Some(0));
    assert_eq!(list.invites[0].claims[0].user_id, user);
    assert_eq!(list.invites[0].issued_by, "Anis");
    assert!(api.audit(100).unwrap().is_empty());
}

#[test]
fn staff_permissions_are_rechecked_and_limits_and_expiry_are_validated() {
    use yo3_server::logins::{self, Login, Registration, Role};
    logins::use_fast_hashing();
    let f = Fixture::new("permissions");
    let mut accounts = f.api.accounts.lock().unwrap();
    let user = accounts.register(Login::new("Croquet", logins::hash_password_now("LongPassword123").unwrap(), "recovery".into()), None, Registration::Open).unwrap();
    accounts.set_role("Croquet", Role::Moderator).unwrap();
    // Existing expired invites stay visible and cannot be used.
    accounts.add_invite(None, Some(logins::now() - 1), "Anis").unwrap();
    drop(accounts);
    let actor = Actor::Staff { user_id: user };
    let first = f.api.create_invite(&request(), ID, actor, "dashboard").unwrap();
    assert_eq!(f.api.list_invites(actor).unwrap().invites[1].issued_by, "Croquet");
    let receipt: String = rusqlite::Connection::open(f.dir.join("server.db")).unwrap().query_row(
        "SELECT result FROM operation_receipts WHERE request_id=?1", [ID], |r| r.get(0)).unwrap();
    assert!(!receipt.contains("Croquet"), "permanent receipts must not retain staff display names: {receipt}");
    assert!(first.expires_at.unwrap().timestamp() > logins::now() + 6 * 86400);
    assert_eq!(f.api.list_invites(actor).unwrap().invites[0].state, InviteState::Expired);
    assert_ne!(f.api.create_invite(&request(), ID, Actor::Host { by: "host" }, "server").unwrap().code, first.code);
    for request in [CreateInviteRequest { uses: Some(0), days: None }, CreateInviteRequest { uses: None, days: Some(0) },
        CreateInviteRequest { uses: None, days: Some(3_000_000) }, CreateInviteRequest { uses: None, days: Some(u32::MAX) }] {
        assert_eq!(f.api.create_invite(&request, "invalid-invite-000001", actor, "dashboard").unwrap_err().code, MutationErrorCode::InvalidRequest);
    }
    assert_eq!(f.api.create_invite(&request(), "bad", actor, "dashboard").unwrap_err().code, MutationErrorCode::InvalidRequest);
    let unlimited = f.api.create_invite(&CreateInviteRequest { uses: None, days: None }, "unlimited-invite-00001", actor, "dashboard").unwrap();
    assert!(unlimited.uses.is_none() && unlimited.expires_at.is_none());
    f.api.accounts.lock().unwrap().set_role("Croquet", Role::Player).unwrap();
    assert_eq!(f.api.create_invite(&request(), ID, actor, "dashboard").unwrap_err().code, MutationErrorCode::Forbidden);
    assert_eq!(f.api.list_invites(actor).unwrap_err().code, MutationErrorCode::Forbidden);
}

#[test]
fn offline_cli_reuses_an_invite_with_the_same_request_id() {
    let f = Fixture::new("cli");
    let run = |by: &str| {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-server")).current_dir(&f.dir)
            .arg("--data-dir").arg(&f.dir).args(["--by", by, "invite", "--uses", "2", "--days", "7", "--request-id", ID]).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stderr).contains("Request ID:"));
        String::from_utf8(output.stdout).unwrap().lines().find(|l| l.starts_with("invite code ")).expect("CLI invite result").to_string()
    };
    assert_eq!(run("Anis"), run("Renamed host"));
    let disk = Directory::open(vec![], &f.dir.join("accounts.json")).unwrap();
    assert_eq!(disk.invites().len(), 1);
    assert_eq!(disk.invites()[0].issued_by, "Anis");
    assert_eq!(f.api.audit(100).unwrap().len(), 1);
}

#[test]
fn an_invalid_legacy_expiry_is_reported_instead_of_being_shown_as_unlimited() {
    let f = Fixture::new("legacy-expiry");
    f.api.accounts.lock().unwrap().add_invite(None, Some(i64::MAX), "host").unwrap();
    assert_eq!(f.api.list_invites(Actor::Host { by: "host" }).unwrap_err().code, MutationErrorCode::PersistenceFailed);
}
