//! Unattended setup can create the first admin, but never reset/re-promote an existing account on a rerun.
use yo3_server::{bootstrap,accounts::Directory,logins::{self,Role},store};
fn root(name:&str)->std::path::PathBuf{let p=std::env::temp_dir().join(format!("yo3-bootstrap-{name}-{}",std::process::id()));let _=std::fs::remove_dir_all(&p);p}
#[tokio::test]
async fn bootstrap_is_atomic_and_never_overwrites_an_existing_password_or_role(){
    logins::use_fast_hashing();let dir=root("rerun");
    assert!(bootstrap::admin(&dir,"HostAdmin","BootstrapPass123".into()).await.unwrap());
    let mut d=Directory::open(vec![],&dir.join("accounts.json")).unwrap();let (_,before)=d.login_of("HostAdmin").unwrap();assert_eq!(before.role,Role::Admin);
    assert!(!bootstrap::admin(&dir,"hostadmin","DifferentPass123".into()).await.unwrap());
    let d2=Directory::open(vec![],&dir.join("accounts.json")).unwrap();assert_eq!(d2.login_of("HostAdmin").unwrap().1.password,before.password);
    assert!(bootstrap::admin(&dir,"OtherAdmin","BootstrapPass123".into()).await.is_err());
    d.set_role("HostAdmin",Role::Player).unwrap();
    assert!(bootstrap::admin(&dir,"HostAdmin","BootstrapPass123".into()).await.is_err(),"a rerun must not undo a deliberate demotion");
    assert!(bootstrap::admin(&dir,"NewAdmin","BootstrapPass123".into()).await.is_err(),"bootstrap closes after its first successful use even if the admin is later demoted");
    store::close(&dir);let _=std::fs::remove_dir_all(dir);
}
#[tokio::test]
async fn bootstrap_failure_leaves_no_partial_admin_and_respects_the_data_lock(){
    logins::use_fast_hashing();let dir=root("failure");store::initialize(&dir).unwrap();
    let conn=rusqlite::Connection::open(dir.join("server.db")).unwrap();
    conn.execute_batch("CREATE TRIGGER refuse_bootstrap BEFORE INSERT ON audit BEGIN SELECT RAISE(ABORT,'audit unavailable'); END;").unwrap();
    assert!(bootstrap::admin(&dir,"HostAdmin","BootstrapPass123".into()).await.is_err());
    assert!(Directory::open(vec![],&dir.join("accounts.json")).unwrap().login_of("HostAdmin").is_none());
    conn.execute_batch("DROP TRIGGER refuse_bootstrap").unwrap();
    let hold=store::lock(&dir).unwrap();assert!(bootstrap::admin(&dir,"HostAdmin","BootstrapPass123".into()).await.is_err());drop(hold);
    assert!(bootstrap::admin(&dir,"HostAdmin","BootstrapPass123".into()).await.unwrap());
    store::close(&dir);let _=std::fs::remove_dir_all(dir);
}
#[tokio::test]
async fn bootstrap_save_receipt_and_commit_failures_roll_back_and_retry_cleanly(){
    logins::use_fast_hashing();
    for (name,trigger) in [
        ("save","CREATE TRIGGER refuse BEFORE INSERT ON items BEGIN SELECT RAISE(ABORT,'save unavailable'); END;"),
        ("receipt","CREATE TRIGGER refuse BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(ABORT,'receipt unavailable'); END;"),
        ("commit","CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER refuse AFTER INSERT ON audit BEGIN INSERT INTO child VALUES(-1); END;")
    ]{
        let dir=root(name);store::initialize(&dir).unwrap();let conn=rusqlite::Connection::open(dir.join("server.db")).unwrap();conn.execute_batch(trigger).unwrap();
        assert!(bootstrap::admin(&dir,"HostAdmin","BootstrapPass123".into()).await.is_err(),"{name}");
        assert!(Directory::open(vec![],&dir.join("accounts.json")).unwrap().login_of("HostAdmin").is_none());
        let audit:u32=conn.query_row("SELECT count(*) FROM audit",[],|r|r.get(0)).unwrap();assert_eq!(audit,0);
        let receipts:u32=conn.query_row("SELECT count(*) FROM operation_receipts",[],|r|r.get(0)).unwrap();assert_eq!(receipts,0);
        conn.execute_batch("DROP TRIGGER refuse").unwrap();assert!(bootstrap::admin(&dir,"HostAdmin","BootstrapPass123".into()).await.unwrap());
        assert!(!bootstrap::admin(&dir,"HostAdmin","DifferentPass123".into()).await.unwrap());
        let stored:String=conn.query_row("SELECT result || input FROM operation_receipts",[],|r|r.get(0)).unwrap();assert!(!stored.contains("BootstrapPass")&&!stored.contains("$argon2"));
        store::close(&dir);let _=std::fs::remove_dir_all(dir);
    }
}
