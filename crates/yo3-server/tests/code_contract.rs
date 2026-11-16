//! BOOSTERPASS administration commits the code, audit and durable retry result before publishing.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use yo3_server::{accounts::Directory, api::{Api, Op, Actor, MutationError, MutationErrorCode, GiveBpRequest,
    codes::{CreateBpCodeRequest, RemoveBpCodeRequest}}, settings, store};

const CODE: &str = "TESTCODE12345678";
const ID: &str = "bp-code-000000000001";
fn create() -> CreateBpCodeRequest { CreateBpCodeRequest { code: Some(CODE.into()), bp: 500, uses: Some(2) } }
struct Fixture { dir: PathBuf, api: Api }
impl Fixture {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("yo3-code-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        store::initialize(&dir).unwrap();
        let accounts = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
        let api = Api::new(Arc::new(Mutex::new(accounts)), Arc::new(settings::Live::open(None, vec![]).unwrap()),
            Some(dir.clone()), Default::default(), "server");
        Self { dir, api }
    }
    fn sql(&self, sql: &str) { rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap(); }
    fn codes(&self) -> (usize, usize) {
        let disk = Directory::open(vec![], &self.dir.join("accounts.json")).unwrap();
        (self.api.accounts.lock().unwrap().codes().len(), disk.codes().len())
    }
    fn change(&self, remove: bool, id: &str) -> Result<serde_json::Value, MutationError> {
        if remove { self.api.remove_bp_code(&RemoveBpCodeRequest { code: CODE.into() }, id, Actor::Host { by: "host" }, "server").map(|r| serde_json::json!(r)) }
        else { self.api.create_bp_code(&create(), id, Actor::Host { by: "host" }, "server").map(|r| serde_json::json!(r)) }
    }
    fn receipts(&self) -> u32 {
        rusqlite::Connection::open(self.dir.join("server.db")).unwrap().query_row("SELECT count(*) FROM operation_receipts", [], |r| r.get(0)).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { store::close(&self.dir); let _ = std::fs::remove_dir_all(&self.dir); }
}

#[test]
fn creating_a_code_cannot_succeed_without_its_audit() {
    let f = Fixture::new("create-audit");
    f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'audit unavailable'); END;");
    let result = f.api.apply(&Op::AddCode { code: CODE.into(), bp: 500, uses: Some(2) }, "host");
    assert!(result.is_err(), "code creation without an audit was reported as done: {result:?}");
    assert_eq!(f.codes(), (0, 0));
}

#[test]
fn removing_a_code_cannot_succeed_without_its_audit() {
    let f = Fixture::new("remove-audit");
    f.api.accounts.lock().unwrap().add_code(CODE, 500, None).unwrap();
    let key = f.api.accounts.lock().unwrap().add_account(&["Yugi".into()]).unwrap();
    let user = f.api.accounts.lock().unwrap().user_of("Yugi").unwrap();
    let (_, balance) = f.api.accounts.lock().unwrap().redeem(user, CODE).unwrap();
    f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'audit unavailable'); END;");
    let result = f.api.apply(&Op::RemoveCode { code: CODE.into() }, "host");
    assert!(result.is_err(), "code removal without an audit was reported as done: {result:?}");
    assert_eq!(f.codes(), (1, 1));
    let list = f.api.list_bp_codes(Actor::Host { by: "host" }).unwrap();
    assert_eq!(list.codes[0].claims[0].user_id, user);
    assert!(list.codes[0].claims[0].at.is_some());
    assert!(!serde_json::to_string(&list).unwrap().contains(&key));
    assert_eq!(f.api.accounts.lock().unwrap().dpoint(user), Some(balance), "removing a code never reclaims credited BP");
}

#[test]
fn save_receipt_and_commit_failures_leave_no_partial_code_or_audit_and_keep_bookkeeping_correct() {
    for failure in ["save", "receipt", "commit"] {
        for remove in [false, true] {
            let f = Fixture::new(&format!("{failure}-{remove}"));
            if remove { f.api.accounts.lock().unwrap().add_code(CODE, 500, None).unwrap(); }
            match failure {
                "save" => store::fail_domain(&f.dir, "accounts.json", true).unwrap(),
                "receipt" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL, 'receipt unavailable'); END;"),
                "commit" => f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY);
                    CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
                    CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES (-1); END;"),
                _ => unreachable!(),
            }
            assert_eq!(f.change(remove, ID).unwrap_err().code, MutationErrorCode::PersistenceFailed, "{failure} {remove}");
            let before = usize::from(remove);
            assert_eq!(f.codes(), (before, before));
            assert!(f.api.audit(100).unwrap().is_empty());
            assert_eq!(f.receipts(), 0);
            if failure == "save" { store::fail_domain(&f.dir, "accounts.json", false).unwrap(); }
            else { f.sql("DROP TRIGGER refuse_change;"); }
            f.api.accounts.lock().unwrap().add_account(&["Yugi".into()]).unwrap();
            assert_eq!(f.codes(), (before, before), "a later save must not publish the refused change");
            f.change(remove, ID).unwrap();
            let after = usize::from(!remove);
            assert_eq!(f.codes(), (after, after));
            assert_eq!(f.api.audit(100).unwrap().len(), 1);
            assert_eq!(f.receipts(), 1);
        }
    }
}

#[test]
fn generated_codes_replay_once_under_concurrency_and_ids_are_shared_with_other_operations() {
    let f = Fixture::new("concurrent");
    let request = CreateBpCodeRequest { code: None, bp: 100, uses: None };
    let results = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8).map(|_| s.spawn(|| f.api.create_bp_code(&request, ID, Actor::Host { by: "host" }, "server").unwrap())).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()
    });
    assert!(results.iter().all(|r| r == &results[0]));
    assert_eq!(results[0].code.len(), 16);
    assert_eq!(f.codes(), (1, 1));
    assert_eq!(f.api.audit(100).unwrap().len(), 1);
    assert_eq!(f.receipts(), 1);
    std::thread::scope(|s| {
        for id in ["bp-code-independent-0001", "bp-code-independent-0002"] {
            s.spawn(|| f.api.create_bp_code(&request, id, Actor::Host { by: "another audit name" }, "server").unwrap());
        }
    });
    assert_eq!(f.codes(), (3, 3));
    assert_eq!(f.receipts(), 3);
    assert_eq!(f.api.give_bp(&GiveBpRequest { who: "Nobody".into(), amount: 1 }, ID, Actor::Host { by: "host" }, "server")
        .unwrap_err().code, MutationErrorCode::RequestConflict);
    assert_eq!(f.api.remove_bp_code(&RemoveBpCodeRequest { code: results[0].code.clone() }, ID, Actor::Host { by: "host" }, "server")
        .unwrap_err().code, MutationErrorCode::RequestConflict);
}

#[test]
fn restart_audit_pruning_and_lost_removal_response_do_not_recreate_or_remove_a_new_code() {
    let f = Fixture::new("restart");
    let first = f.api.create_bp_code(&create(), ID, Actor::Host { by: "host" }, "server").unwrap();
    let remove = RemoveBpCodeRequest { code: CODE.to_lowercase() };
    let removed = f.api.remove_bp_code(&remove, "remove-code-000000001", Actor::Host { by: "host" }, "server").unwrap();
    // Reusing a removed code is allowed as a NEW intentional change. An old removal retry must not remove it.
    f.api.create_bp_code(&create(), "create-code-000000002", Actor::Host { by: "host" }, "server").unwrap();
    f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';");
    store::prune_audit(&f.dir, 1).unwrap();
    store::close(&f.dir);
    store::initialize(&f.dir).unwrap();
    let api = Api::new(Arc::new(Mutex::new(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap())),
        Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(f.dir.clone()), Default::default(), "offline");
    assert_eq!(api.create_bp_code(&create(), ID, Actor::Host { by: "renamed host" }, "offline").unwrap(), first);
    assert_eq!(api.remove_bp_code(&remove, "remove-code-000000001", Actor::Host { by: "host" }, "offline").unwrap(), removed);
    assert_eq!(api.accounts.lock().unwrap().codes().len(), 1);
    assert!(api.audit(100).unwrap().is_empty());
    assert_eq!(f.receipts(), 3);
}

#[test]
fn permissions_are_checked_before_replay_and_staff_and_host_receipts_have_distinct_scopes() {
    use yo3_server::logins::{self, Login, Registration, Role};
    logins::use_fast_hashing();
    let f = Fixture::new("permissions");
    let mut accounts = f.api.accounts.lock().unwrap();
    let user = accounts.register(Login::new("Pegasus", logins::hash_password_now("LongPassword123").unwrap(), "recovery".into()), None, Registration::Open).unwrap();
    accounts.set_role("Pegasus", Role::Admin).unwrap();
    drop(accounts);
    let request = CreateBpCodeRequest { code: None, bp: 100, uses: None };
    let staff = f.api.create_bp_code(&request, ID, Actor::Staff { user_id: user }, "dashboard").unwrap();
    let host = f.api.create_bp_code(&request, ID, Actor::Host { by: "host" }, "server").unwrap();
    assert_ne!(staff.code, host.code);
    f.api.accounts.lock().unwrap().set_role("Pegasus", Role::Moderator).unwrap();
    assert_eq!(f.api.create_bp_code(&request, ID, Actor::Staff { user_id: user }, "dashboard").unwrap_err().code, MutationErrorCode::Forbidden);
    assert_eq!(f.api.remove_bp_code(&RemoveBpCodeRequest { code: host.code }, "remove-code-000000001", Actor::Staff { user_id: user }, "dashboard").unwrap_err().code, MutationErrorCode::Forbidden);
    assert_eq!(f.api.list_bp_codes(Actor::Staff { user_id: user }).unwrap().codes.len(), 2);
    f.api.accounts.lock().unwrap().set_role("Pegasus", Role::Player).unwrap();
    assert_eq!(f.api.list_bp_codes(Actor::Staff { user_id: user }).unwrap_err().code, MutationErrorCode::Forbidden);
}

#[test]
fn validated_input_normalizes_codes_and_reports_duplicate_missing_and_invalid_values() {
    let f = Fixture::new("validation");
    let first = f.change(false, ID).unwrap();
    let lower = CreateBpCodeRequest { code: Some(CODE.to_lowercase()), ..create() };
    assert_eq!(serde_json::to_value(f.api.create_bp_code(&lower, ID, Actor::Host { by: "host" }, "server").unwrap()).unwrap(), first);
    assert_eq!(f.api.create_bp_code(&lower, "another-code-000000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::AlreadyExists);
    for request in [CreateBpCodeRequest { bp: 0, ..create() }, CreateBpCodeRequest { uses: Some(0), ..create() },
        CreateBpCodeRequest { code: Some("wrong".into()), ..create() }] {
        assert_eq!(f.api.create_bp_code(&request, "another-code-000000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
    }
    assert_eq!(f.change(true, "invalid\r\n").unwrap_err().code, MutationErrorCode::InvalidRequest);
    assert_eq!(f.api.remove_bp_code(&RemoveBpCodeRequest { code: "OTHER00000000001".into() }, "another-code-000000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::NotFound);
    assert_eq!(f.codes(), (1, 1));
    assert_eq!(f.receipts(), 1);
}

#[test]
fn cli_retries_use_the_original_receipt_even_after_the_host_changes_the_audit_name() {
    let f = Fixture::new("cli");
    let run = |args: &[&str], by: &str| {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-server")).current_dir(&f.dir)
            .arg("--data-dir").arg(&f.dir).args(["--by", by]).args(args).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stderr).contains("Request ID:"));
        String::from_utf8(output.stdout).unwrap().lines().find(|l| l.starts_with("code ")).expect("CLI result").to_string()
    };
    let args = ["add-code", "", "300", "--request-id", ID];
    let first = run(&args, "Anis");
    assert_eq!(run(&args, "Another name"), first);
    let disk = Directory::open(vec![], &f.dir.join("accounts.json")).unwrap();
    assert_eq!(disk.codes().len(), 1);
    let code = &disk.codes()[0].code;
    let args = ["remove-code", code.as_str(), "--request-id", "remove-code-cli-000001"];
    let removed = run(&args, "Anis");
    assert_eq!(run(&args, "Another name"), removed);
    assert_eq!(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap().codes().len(), 0);
    assert_eq!(f.receipts(), 2);
    assert_eq!(f.api.audit(100).unwrap().len(), 2);
}
