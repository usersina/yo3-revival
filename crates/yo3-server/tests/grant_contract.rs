//! A host's BP grant is one durable change: balance, D-Mail, audit and retry receipt.
mod common;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use yo3_server::{accounts::{Directory, START_BP}, api::{self, Api, Op, GiveBpRequest, Actor, MutationErrorCode}, settings, store};

const ID: &str = "grant-000000000001";
fn request(amount: u32) -> GiveBpRequest { GiveBpRequest { who: "Yugi".into(), amount } }

struct Fixture {
    dir: PathBuf,
    api: Api,
    client: common::Client,
    user: u32,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("yo3-grant-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        store::initialize(&dir).unwrap();
        let mut accounts = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
        accounts.add_account(&["Yugi".into()]).unwrap();
        let user = accounts.user_of("Yugi").unwrap();
        let accounts = Arc::new(Mutex::new(accounts));
        let server = common::Server::new();
        *server.hub.lock().unwrap() = yo3_server::front::Hub::with_records(&dir.join("records.json"));
        server.hub.lock().unwrap().set_directory(accounts.clone());
        server.hub.lock().unwrap().keep_mail(&dir.join("mail.json"));
        let mut client = server.join_saved(&accounts, user);
        client.pushed();
        let mut api = Api::new(accounts, Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(dir.clone()), PathBuf::new(), "server");
        api.hub = Some(server.hub);
        Self { dir, api, client, user }
    }

    fn sql(&self, sql: &str) {
        rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap();
    }

    fn balance(&self) -> (u32, u32) {
        let disk = Directory::open(vec![], &self.dir.join("accounts.json")).unwrap();
        (self.api.accounts.lock().unwrap().dpoint(self.user).unwrap(), disk.dpoint(self.user).unwrap())
    }

    fn mail_count(&mut self) -> u32 {
        common::one(self.client.send(common::req(0x2583))).get("count").and_then(yo3_proto::Value::as_u32).unwrap()
    }

    fn receipts(&self) -> u32 {
        rusqlite::Connection::open(self.dir.join("server.db")).unwrap().query_row("SELECT count(*) FROM operation_receipts", [], |r| r.get(0)).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        store::close(&self.dir);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn an_audit_failure_does_not_give_bp_or_send_mail() {
    let mut f = Fixture::new("audit-failure");
    f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'audit unavailable'); END;");
    let result = f.api.apply(&Op::GiveBp { who: "Yugi".into(), amount: 250 }, "host");
    assert!(result.is_err(), "a grant without an audit entry was reported as done: {result:?}");
    assert_eq!(f.balance(), (START_BP, START_BP));
    assert!(f.client.pushed().is_empty(), "no balance or mail notification before commit");
    assert_eq!((f.mail_count(), f.receipts()), (0, 0));
    assert!(f.api.audit(100).unwrap().is_empty());
    f.sql("DROP TRIGGER refuse_audit;");
    f.api.apply(&Op::GiveBp { who: "Yugi".into(), amount: 10 }, "host").unwrap();
    assert_eq!(f.balance(), (START_BP + 10, START_BP + 10), "a later save must not resurrect the failed grant");
}

#[test]
fn a_mail_failure_does_not_give_bp_or_write_an_audit_entry() {
    let mut f = Fixture::new("mail-failure");
    store::fail_domain(&f.dir, "mail.json", true).unwrap();
    let result = f.api.apply(&Op::GiveBp { who: "Yugi".into(), amount: 250 }, "host");
    assert!(result.is_err(), "a grant without its mail was reported as done: {result:?}");
    assert_eq!(f.balance(), (START_BP, START_BP));
    assert!(f.client.pushed().is_empty());
    assert_eq!((f.mail_count(), f.receipts()), (0, 0));
    assert!(f.api.audit(100).unwrap().is_empty());
}

#[test]
fn a_deferred_constraint_failure_at_commit_publishes_nothing_and_can_be_retried() {
    let mut f = Fixture::new("commit-failure");
    // Real SQLite COMMIT failure: the statements succeed, but their deferred foreign key cannot commit.
    f.sql("CREATE TABLE test_parent(id INTEGER PRIMARY KEY);
        CREATE TABLE test_child(parent INTEGER REFERENCES test_parent(id) DEFERRABLE INITIALLY DEFERRED);
        CREATE TRIGGER refuse_commit AFTER INSERT ON audit BEGIN INSERT INTO test_child VALUES (-1); END;");
    let error = f.api.give_bp(&request(250), ID, Actor::Host { by: "host" }, "server").unwrap_err();
    assert_eq!(error.code, MutationErrorCode::PersistenceFailed);
    assert_eq!(f.balance(), (START_BP, START_BP));
    assert!(f.client.pushed().is_empty());
    assert_eq!((f.mail_count(), f.receipts()), (0, 0));
    assert!(f.api.audit(100).unwrap().is_empty());
    f.sql("DROP TRIGGER refuse_commit;");
    let result = f.api.give_bp(&request(250), ID, Actor::Host { by: "host" }, "server").unwrap();
    assert_eq!(result.bp, START_BP + 250);
    assert_eq!(f.balance(), (result.bp, result.bp));
    assert_eq!((f.mail_count(), f.receipts()), (1, 1));
}

#[test]
fn a_receipt_write_failure_rolls_back_the_entire_grant() {
    let mut f = Fixture::new("receipt-failure");
    f.sql("CREATE TRIGGER refuse_receipt BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL, 'receipt unavailable'); END;");
    assert_eq!(f.api.give_bp(&request(25), ID, Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::PersistenceFailed);
    assert_eq!(f.balance(), (START_BP, START_BP));
    assert_eq!((f.mail_count(), f.receipts()), (0, 0));
    assert!(f.api.audit(100).unwrap().is_empty());
    assert!(f.client.pushed().is_empty());
}

#[test]
fn retrying_a_lost_answer_does_not_repeat_mail_notifications_or_audit_even_after_pruning() {
    let mut f = Fixture::new("retry");
    let first = f.api.give_bp(&request(250), ID, Actor::Host { by: "Anis" }, "server").unwrap();
    assert_eq!(f.balance(), (first.bp, first.bp));
    assert_eq!((f.mail_count(), f.receipts()), (1, 1));
    let messages = f.client.pushed();
    assert!(messages.iter().any(|m| m.request() == Some(0x2581)), "mail bell only after commit");
    let letters = store::read(&f.dir.join("mail.json")).unwrap().unwrap();
    assert_eq!(letters["waiting"].as_array().unwrap().len(), 1);
    assert_eq!(f.api.audit(100).unwrap().len(), 1);
    // --by changes only the audit label, never the fixed host receipt identity.
    assert_eq!(f.api.give_bp(&request(250), ID, Actor::Host { by: "Another name" }, "offline").unwrap(), first);
    assert!(f.client.pushed().is_empty());
    f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';");
    assert_eq!(store::prune_audit(&f.dir, 1).unwrap(), 1);
    assert_eq!(f.api.give_bp(&request(250), ID, Actor::Host { by: "host" }, "server").unwrap(), first);
    assert_eq!(f.balance(), (first.bp, first.bp));
    assert_eq!((f.mail_count(), f.receipts()), (1, 1));
    assert!(f.api.audit(100).unwrap().is_empty());
    assert!(f.client.pushed().is_empty());
    assert_eq!(f.api.give_bp(&request(251), ID, Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::RequestConflict);
}

#[test]
fn receipts_and_mail_survive_a_restart() {
    let f = Fixture::new("restart");
    let first = f.api.give_bp(&request(100), ID, Actor::Host { by: "host" }, "server").unwrap();
    store::close(&f.dir);
    store::initialize(&f.dir).unwrap();
    let accounts = Arc::new(Mutex::new(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap()));
    let restarted = Api::new(accounts, Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(f.dir.clone()), PathBuf::new(), "offline");
    assert_eq!(restarted.give_bp(&request(100), ID, Actor::Host { by: "host" }, "offline").unwrap(), first);
    assert_eq!(restarted.accounts.lock().unwrap().dpoint(f.user), Some(first.bp));
    assert_eq!(restarted.audit(100).unwrap().len(), 1);
    assert_eq!(store::read(&f.dir.join("mail.json")).unwrap().unwrap()["waiting"].as_array().unwrap().len(), 1);
}

#[test]
fn concurrent_retries_and_duel_rewards_keep_the_same_lock_order() {
    let mut f = Fixture::new("concurrent");
    std::thread::scope(|s| {
        let api = &f.api;
        let reward = api.hub.as_ref().unwrap();
        let player = api.accounts.lock().unwrap().player_id_named("Yugi").unwrap();
        s.spawn(move || for _ in 0..20 { reward.lock().unwrap().record_result(player, 7, 1); });
        for _ in 0..8 {
            s.spawn(move || api.give_bp(&request(100), ID, Actor::Host { by: "host" }, "server").unwrap());
        }
    });
    let expected = START_BP + 100 + 20 * yo3_server::shop::duel_bp(1);
    assert_eq!(f.balance(), (expected, expected));
    assert_eq!((f.mail_count(), f.receipts()), (1, 1));
    assert_eq!(f.api.audit(100).unwrap().len(), 1);
}

#[test]
fn staff_receipts_are_scoped_by_account_id_and_permissions_are_rechecked_before_replay() {
    let f = Fixture::new("staff");
    let mut accounts = f.api.accounts.lock().unwrap();
    let id = accounts.register(yo3_server::logins::Login::new("Pegasus", "test-hash".into(), "test-recovery".into()), None, yo3_server::logins::Registration::Open).unwrap();
    accounts.set_role("Pegasus", yo3_server::logins::Role::Admin).unwrap();
    drop(accounts);
    f.api.give_bp(&request(10), ID, Actor::Host { by: "host" }, "server").unwrap();
    let staff = f.api.give_bp(&request(10), ID, Actor::Staff { user_id: id }, "panel").unwrap();
    assert_eq!(staff.bp, START_BP + 20);
    assert_eq!(f.receipts(), 2);
    f.api.accounts.lock().unwrap().set_role("Pegasus", yo3_server::logins::Role::Moderator).unwrap();
    assert_eq!(f.api.give_bp(&request(10), ID, Actor::Staff { user_id: id }, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    assert_eq!(f.balance(), (staff.bp, staff.bp));
}

#[test]
fn the_local_http_contract_requires_a_safe_key_and_returns_typed_conflicts() {
    let f = Fixture::new("http");
    let send = |key: &str, amount| {
        let body = serde_json::json!({ "op": "give-bp", "who": "Yugi", "amount": amount }).to_string();
        let raw = format!("POST /api/admin/v1/give-bp HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer secret\r\n{key}\r\n{body}");
        api::handle(&raw, "127.0.0.1".parse().unwrap(), Some("secret"), &f.api).unwrap()
    };
    assert_eq!(send("", 10).status, "400 Bad Request");
    assert_eq!(send("Idempotency-Key: short\r\n", 10).status, "400 Bad Request");
    let key = format!("Idempotency-Key: {ID}\r\n");
    assert_eq!(send(&format!("{key}{key}"), 10).status, "400 Bad Request");
    let first = send(&key, 10);
    assert_eq!(first.status, "200 OK");
    assert_eq!(send(&key, 10).body, first.body);
    let conflict = send(&key, 11);
    assert_eq!(conflict.status, "409 Conflict");
    assert_eq!(serde_json::from_slice::<serde_json::Value>(&conflict.body).unwrap()["code"], "request_conflict");
    assert_eq!(f.balance(), (START_BP + 10, START_BP + 10));
}

#[test]
fn schema_two_upgrades_without_losing_accounts_mail_or_audit() {
    let f = Fixture::new("migration");
    f.api.give_bp(&request(10), ID, Actor::Host { by: "host" }, "server").unwrap();
    f.sql("DROP TABLE operation_receipts; PRAGMA user_version=2;");
    store::close(&f.dir);
    store::initialize(&f.dir).unwrap();
    assert_eq!(f.balance(), (START_BP + 10, START_BP + 10));
    assert_eq!(f.receipts(), 0);
    assert_eq!(f.api.audit(100).unwrap().len(), 1);
    assert_eq!(store::read(&f.dir.join("mail.json")).unwrap().unwrap()["waiting"].as_array().unwrap().len(), 1);
}

#[test]
fn long_lived_receipts_do_not_retain_player_names() {
    let f = Fixture::new("receipt-privacy");
    f.api.give_bp(&request(10), ID, Actor::Host { by: "host" }, "server").unwrap();
    let text: String = rusqlite::Connection::open(f.dir.join("server.db")).unwrap()
        .query_row("SELECT input || result FROM operation_receipts", [], |r| r.get(0)).unwrap();
    assert!(!text.contains("Yugi"), "receipts survive audit pruning and account deletion: {text}");
}

#[test]
fn invalid_grants_change_nothing() {
    let mut f = Fixture::new("invalid");
    for (input, key, code) in [
        (request(0), ID, MutationErrorCode::InvalidRequest),
        (request(u32::MAX), ID, MutationErrorCode::InvalidRequest),
        (request(1), "bad\r\nkey", MutationErrorCode::InvalidRequest),
        (GiveBpRequest { who: "Nobody".into(), amount: 1 }, ID, MutationErrorCode::NotFound),
    ] {
        assert_eq!(f.api.give_bp(&input, key, Actor::Host { by: "host" }, "server").unwrap_err().code, code);
    }
    assert_eq!(f.balance(), (START_BP, START_BP));
    assert_eq!((f.mail_count(), f.receipts()), (0, 0));
    assert!(f.api.audit(100).unwrap().is_empty());
    assert!(f.client.pushed().is_empty());
}

#[test]
fn the_cli_reuses_the_host_receipt_even_when_by_changes() {
    let f = Fixture::new("cli");
    let run = |by| std::process::Command::new(env!("CARGO_BIN_EXE_yo3-server"))
        .arg("--data-dir").arg(&f.dir).arg("--by").arg(by).args(["give-bp", "Yugi", "25", "--request-id", ID]).output().unwrap();
    let first = run("Anis");
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    let retry = run("Another name");
    assert!(retry.status.success(), "{}", String::from_utf8_lossy(&retry.stderr));
    assert_eq!(String::from_utf8_lossy(&retry.stdout).lines().last(), String::from_utf8_lossy(&first.stdout).lines().last(),
        "the same result; the first call also logs its commit");
    assert!(String::from_utf8_lossy(&first.stderr).contains(ID), "the retry ID is printed before sending");
    assert_eq!(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap().dpoint(f.user), Some(START_BP + 25));
    assert_eq!(f.receipts(), 1);
    assert_eq!(f.api.audit(100).unwrap().len(), 1);
    assert_eq!(f.api.audit(100).unwrap()[0].actor, "Anis");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_retry_over_http_after_discarding_the_answer_and_restarting_does_not_grant_twice() {
    use tokio::io::AsyncWriteExt;
    let root = std::env::temp_dir().join(format!("yo3-grant-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let config = yo3_server::Config::ephemeral(root);
    let dir = config.data_dir.clone().unwrap();
    let server = yo3_server::start(config.clone()).await.unwrap();
    let live = yo3_server::admin::running(&dir).unwrap();
    let setup = live.clone();
    tokio::task::spawn_blocking(move || api::call(&setup, &Op::CreateAccount { names: vec!["Yugi".into()] })).await.unwrap().unwrap();
    let body = r#"{"op":"give-bp","who":"Yugi","amount":75}"#;
    let raw = format!("POST /api/admin/v1/give-bp HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {}\r\nIdempotency-Key: {ID}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", live.token, body.len());
    let mut stream = tokio::net::TcpStream::connect(server.http).await.unwrap();
    stream.write_all(raw.as_bytes()).await.unwrap();
    stream.shutdown().await.unwrap();
    drop(stream); // The client never sees the result.
    let polling = live.clone();
    tokio::task::spawn_blocking(move || {
        let until = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let audit = api::call(&polling, &Op::Audit { limit: None }).unwrap();
            if audit["rows"].as_array().unwrap().iter().any(|r| r["op"] == "give-bp") { break }
            assert!(std::time::Instant::now() < until, "first request did not commit");
            std::thread::yield_now();
        }
        api::call(&polling, &Op::Shutdown).unwrap();
    }).await.unwrap();
    server.wait().await;
    let server = yo3_server::start(config).await.unwrap();
    let live = yo3_server::admin::running(&dir).unwrap();
    tokio::task::spawn_blocking(move || {
        let result = api::call_as_request(&live, &Op::GiveBp { who: "Yugi".into(), amount: 75 }, "Changed display name", ID).unwrap();
        assert_eq!(result["bp"], START_BP + 75);
        let accounts = api::call(&live, &Op::Accounts).unwrap();
        let target = accounts.as_array().unwrap().iter().find(|a| a["user_id"] == result["user_id"]).unwrap();
        assert_eq!(target["bp"], START_BP + 75);
        let audit = api::call(&live, &Op::Audit { limit: None }).unwrap();
        assert_eq!(audit["rows"].as_array().unwrap().iter().filter(|r| r["op"] == "give-bp").count(), 1);
        assert_eq!(store::read(&dir.join("mail.json")).unwrap().unwrap()["waiting"].as_array().unwrap().len(), 1);
        api::call(&live, &Op::Shutdown).unwrap();
    }).await.unwrap();
    server.wait().await;
}
