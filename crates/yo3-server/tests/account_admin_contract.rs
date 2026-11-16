//! Account profile administration: no change is successful without its audit and durable retry receipt.
use std::{path::PathBuf, sync::{Arc, Mutex}};
use serde_json::{Value, json};
use yo3_server::{accounts::Directory, api::{Api, Op, Actor, MutationErrorCode, profiles::AccountEditRequest}, settings, store};

struct Fixture { dir: PathBuf, api: Api, user: u32, player: u32 }
impl Fixture {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("yo3-account-admin-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        store::initialize(&dir).unwrap();
        let mut d = Directory::open(vec![], &dir.join("accounts.json")).unwrap();
        d.add_account(&["Yugi".into()]).unwrap();
        let user = d.user_of("Yugi").unwrap();
        let player = d.player_id_named("Yugi").unwrap();
        d.set_country_of(user, "FR").unwrap();
        let api = Api::new(Arc::new(Mutex::new(d)), Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(dir.clone()), Default::default(), "server");
        Self { dir, api, user, player }
    }
    fn sql(&self, sql: &str) { rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap(); }
    fn state(&self, disk: bool) -> Value {
        let disk = disk.then(|| Directory::open(vec![], &self.dir.join("accounts.json")).unwrap());
        let d = self.api.accounts.lock().unwrap();
        let a = disk.as_ref().unwrap_or(&d).account_of(self.user).unwrap();
        json!({ "country": a.country, "players": a.players.iter().map(|p| json!({ "id": p.player_id, "name": p.name, "slot": p.index, "country": p.country })).collect::<Vec<_>>() })
    }
}
impl Drop for Fixture { fn drop(&mut self) { store::close(&self.dir); let _ = std::fs::remove_dir_all(&self.dir); } }
fn operations() -> [Op; 3] { [Op::AddPlayer { who: "Yugi".into(), name: "Joey".into() }, Op::Rename { old: "Yugi".into(), new: "Atem".into() }, Op::SetCountry { who: "Yugi".into(), code: "auto".into() }] }

#[test]
fn account_changes_cannot_succeed_without_their_audit() {
    for op in operations() {
        let f = Fixture::new(&op.name());
        let before = f.state(false);
        f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'no audit'); END;");
        let result = f.api.apply(&op, "host");
        assert!(result.is_err(), "{} reported success without audit: {result:?}", op.name());
        assert_eq!(f.state(false), before);
        assert_eq!(f.state(true), before);
    }
}

#[test]
fn retry_after_rename_resolves_the_original_receipt_even_when_the_old_name_is_gone() {
    let f = Fixture::new("rename-retry");
    let op = Op::Rename { old: "Yugi".into(), new: "Atem".into() };
    let first = f.api.apply_request(&op, "Anis", "server", None, "rename-player-00000001").unwrap();
    assert_eq!(f.api.apply_request(&op, "Another audit name", "offline", None, "rename-player-00000001").unwrap(), first);
    assert_eq!(f.api.audit(100).unwrap().len(), 1);
}

const ID: &str = "account-edit-00000001";
fn edits(f: &Fixture) -> [AccountEditRequest; 3] { [
    AccountEditRequest::AddPlayer { user_id: f.user, name: "Joey".into() },
    AccountEditRequest::RenamePlayer { user_id: f.user, player_id: f.player, name: "Atem".into() },
    AccountEditRequest::SetCountry { user_id: f.user, country: None },
] }

#[test]
fn save_receipt_and_commit_failures_roll_back_all_profile_edits_and_later_saves() {
    for failure in ["save", "receipt", "commit"] {
        for index in 0..3 {
            let f = Fixture::new(&format!("{failure}-{index}"));
            let request = edits(&f)[index].clone();
            let before = f.state(false);
            match failure {
                "save" => store::fail_domain(&f.dir, "accounts.json", true).unwrap(),
                "receipt" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL, 'no receipt'); END;"),
                "commit" => f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
                    CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES (-1); END;"),
                _ => unreachable!(),
            }
            assert_eq!(f.api.edit_account(&request, ID, Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::PersistenceFailed);
            assert_eq!(f.state(false), before); assert_eq!(f.state(true), before);
            assert!(f.api.audit(100).unwrap().is_empty());
            if failure == "save" { store::fail_domain(&f.dir, "accounts.json", false).unwrap(); } else { f.sql("DROP TRIGGER refuse_change;"); }
            f.api.accounts.lock().unwrap().add_account(&["Other".into()]).unwrap();
            assert_eq!(f.api.accounts.lock().unwrap().player_id_named("Other"), Some(f.player + 1), "failed edits cannot consume player IDs");
            assert_eq!(f.state(false), before); assert_eq!(f.state(true), before);
            f.api.edit_account(&request, ID, Actor::Host { by: "host" }, "server").unwrap();
            assert_eq!(f.state(false), f.state(true)); assert_ne!(f.state(false), before);
            assert_eq!(f.api.audit(100).unwrap().len(), 1);
            assert_eq!(rusqlite::Connection::open(f.dir.join("server.db")).unwrap().query_row("SELECT count(*) FROM operation_receipts", [], |r| r.get::<_, u32>(0)).unwrap(), 1);
        }
    }
}

#[test]
fn concurrent_player_creation_replays_once_and_independent_edits_keep_all_four_slots() {
    let f = Fixture::new("concurrent");
    let request = edits(&f)[0].clone();
    let results = std::thread::scope(|s| {
        let jobs: Vec<_> = (0..8).map(|_| s.spawn(|| f.api.edit_account(&request, ID, Actor::Host { by: "host" }, "server").unwrap())).collect();
        jobs.into_iter().map(|j| j.join().unwrap()).collect::<Vec<_>>()
    });
    assert!(results.iter().all(|r| r == &results[0]));
    std::thread::scope(|s| {
        for (name, id) in [("Kaiba", "player-independent-0001"), ("Mai", "player-independent-0002")] {
            let f = &f;
            s.spawn(move || f.api.edit_account(&AccountEditRequest::AddPlayer { user_id: f.user, name: name.into() }, id, Actor::Host { by: "host" }, "server").unwrap());
        }
    });
    let account = f.api.account_detail(f.user, Actor::Host { by: "host" }).unwrap();
    assert_eq!(account.players.len(), 4);
    let slots: std::collections::BTreeSet<_> = account.players.iter().map(|p| p.slot).collect();
    assert_eq!(slots.into_iter().collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert_eq!(f.api.audit(100).unwrap().len(), 3);
    assert_eq!(f.api.edit_account(&AccountEditRequest::AddPlayer { user_id: f.user, name: "Tea".into() }, "over-full-00000000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::RequestConflict);
}

#[test]
fn restart_and_pruning_preserve_receipts_without_reapplying_old_country_or_rename_changes() {
    let f = Fixture::new("restart");
    let request = edits(&f)[1].clone();
    let first = f.api.edit_account(&request, ID, Actor::Host { by: "Anis" }, "server").unwrap();
    f.api.accounts.lock().unwrap().rename("Atem", "Yami").unwrap();
    f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';"); store::prune_audit(&f.dir, 1).unwrap();
    store::close(&f.dir); store::initialize(&f.dir).unwrap();
    let api = Api::new(Arc::new(Mutex::new(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap())), Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(f.dir.clone()), Default::default(), "offline");
    assert_eq!(api.edit_account(&request, ID, Actor::Host { by: "another audit name" }, "offline").unwrap(), first);
    assert_eq!(api.accounts.lock().unwrap().player_id_named("Yami"), Some(f.player));
    assert!(api.audit(100).unwrap().is_empty());
    assert_eq!(api.edit_account(&edits(&f)[2], ID, Actor::Host { by: "host" }, "offline").unwrap_err().code, MutationErrorCode::RequestConflict);
    let payload: String = rusqlite::Connection::open(f.dir.join("server.db")).unwrap().query_row("SELECT result FROM operation_receipts", [], |r| r.get(0)).unwrap();
    for private in ["Anis", "Yugi", "Atem", "Yami"] { assert!(!payload.contains(private)); }
}

#[test]
fn moderator_target_permissions_are_rechecked_on_replay_and_names_are_validated() {
    use yo3_server::logins::{self, Login, Registration, Role};
    logins::use_fast_hashing();
    let f = Fixture::new("permissions");
    let mut d = f.api.accounts.lock().unwrap();
    let moderator = d.register(Login::new("Croquet", logins::hash_password_now("LongPassword123").unwrap(), "secret".into()), None, Registration::Open).unwrap();
    let target = d.register(Login::new("Pegasus", logins::hash_password_now("LongPassword123").unwrap(), "secret".into()), None, Registration::Open).unwrap();
    d.set_role("Croquet", Role::Moderator).unwrap(); d.add_player_to(target, "Pegasus").unwrap();
    let player_id = d.player_id_named("Pegasus").unwrap(); drop(d);
    let request = AccountEditRequest::RenamePlayer { user_id: target, player_id, name: "Maximillion".into() };
    let actor = Actor::Staff { user_id: moderator };
    f.api.edit_account(&request, ID, actor, "panel").unwrap();
    f.api.accounts.lock().unwrap().set_role("Pegasus", Role::Admin).unwrap();
    assert_eq!(f.api.edit_account(&request, ID, actor, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    for request in [edits(&f)[0].clone(), edits(&f)[2].clone()] {
        assert_eq!(f.api.edit_account(&request, "moderator-edit-000001", actor, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    }
    for name in ["", "                 ", "12345678901234567", "bad\nname"] {
        let req = AccountEditRequest::AddPlayer { user_id: f.user, name: name.into() };
        assert_eq!(f.api.edit_account(&req, "invalid-name-00000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
    }
    assert_eq!(f.api.edit_account(&AccountEditRequest::RenamePlayer { user_id: f.user, player_id, name: "Joey".into() }, "wrong-target-00000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::NotFound);
    f.api.accounts.lock().unwrap().set_role("Croquet", Role::Player).unwrap();
    assert_eq!(f.api.account_detail(f.user, actor).unwrap_err().code, MutationErrorCode::Forbidden);
}

#[test]
fn account_details_exclude_credentials_and_preserve_large_collection_counts_and_unknown_live_state() {
    let f = Fixture::new("projection");
    f.api.accounts.lock().unwrap().update_player(f.player, |p| { p.cards.insert(1, u32::MAX); p.cards.insert(2, u32::MAX); }).unwrap();
    let detail = f.api.account_detail(f.user, Actor::Host { by: "host" }).unwrap();
    assert!(detail.has_key); assert!(detail.login.is_none()); assert_eq!(detail.country.as_deref(), Some("FR"));
    assert_eq!(detail.players[0].additional_cards, u64::from(u32::MAX) * 2);
    assert!(detail.players[0].online.is_none()); assert!(detail.players[0].record.is_none());
    let encoded = serde_json::to_value(detail).unwrap();
    for field in ["key", "password", "recovery", "reset", "devices", "token", "avatar", "pbin"] { assert!(!encoded.as_object().unwrap().contains_key(field)); }
    assert_eq!(f.api.account_detail(u32::MAX, Actor::Host { by: "host" }).unwrap_err().code, MutationErrorCode::NotFound);
    assert_eq!(f.api.account_countries(Actor::Host { by: "host" }).unwrap_err().code, MutationErrorCode::PersistenceFailed);
}

#[test]
fn account_details_project_real_password_accounts_without_any_authentication_material() {
    use yo3_server::logins::{self, Login, Device, Mute, Registration};
    logins::use_fast_hashing();
    let f = Fixture::new("credentials");
    let mut login = Login::new("SecretTarget", logins::hash_password_now("PrivatePassword123").unwrap(), "PRIVATE_RECOVERY".into());
    let password_hash = login.password.clone();
    let now = logins::now();
    login.reset = Some(("PRIVATE_RESET".into(), now + 3600));
    login.devices.push(Device { token: "PRIVATE_DEVICE".into(), label: "Private laptop".into(), created: now, last_used: now, expires: now + 3600 });
    login.muted = Some(Mute { reason: "Chat rule".into(), until: Some(now + 3600) });
    let user = f.api.accounts.lock().unwrap().register(login, None, Registration::Open).unwrap();
    let detail = f.api.account_detail(user, Actor::Host { by: "host" }).unwrap();
    assert_eq!(detail.device_count, 1); assert!(detail.reset_pending);
    assert!(detail.muted.as_ref().unwrap().until.is_some());
    let encoded = serde_json::to_string(&detail).unwrap();
    for secret in [password_hash.as_str(), "PrivatePassword123", "PRIVATE_RECOVERY", "PRIVATE_RESET", "PRIVATE_DEVICE", "Private laptop"] {
        assert!(!encoded.contains(secret), "account details leaked {secret}");
    }
}

#[test]
fn offline_cli_profile_retries_keep_their_ids_and_never_repeat_a_change() {
    let f = Fixture::new("cli");
    let run = |args: &[&str]| {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-server")).current_dir(&f.dir).arg("--data-dir").arg(&f.dir).args(args).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stderr).contains("Request ID:"));
        String::from_utf8(output.stdout).unwrap()
    };
    for args in [vec!["add-player", "Yugi", "Joey", "--request-id", "cli-profile-add-00001"],
        vec!["rename", "Yugi", "Atem", "--request-id", "cli-profile-rename-0001"],
        vec!["set-country", "Atem", "auto", "--request-id", "cli-profile-country-0001"]] {
        assert_eq!(run(&args), run(&args));
    }
    let d = Directory::open(vec![], &f.dir.join("accounts.json")).unwrap();
    assert_eq!(d.account_of(f.user).unwrap().players.len(), 2);
    assert_eq!(d.player_id_named("Atem"), Some(f.player));
    assert_eq!(d.account_of(f.user).unwrap().country, "");
    assert_eq!(f.api.audit(100).unwrap().len(), 3);
}

#[test]
fn country_choices_and_validation_use_the_archive_instead_of_an_invented_allowlist() {
    let mut f = Fixture::new("countries");
    f.api.game_dir = f.dir.join("synthetic-game");
    // A synthetic, uncompressed YPK entry containing two text-table rows. No original game data is copied.
    let name = b"data/text/countrytbl.bin";
    let mut table = vec![0u8; 24];
    table[0..4].copy_from_slice(&2u32.to_le_bytes());
    table[4..8].copy_from_slice(&1u32.to_le_bytes());
    table[12..16].copy_from_slice(&2u32.to_le_bytes());
    table[16..20].copy_from_slice(&6u32.to_le_bytes());
    for unit in "FR\0JP\0".encode_utf16() { table.extend_from_slice(&unit.to_le_bytes()); }
    let mut archive = vec![0u8; 30];
    archive[..4].copy_from_slice(b"YO\x03\x04");
    archive[18..22].copy_from_slice(&(table.len() as u32).to_le_bytes());
    archive[22..26].copy_from_slice(&(table.len() as u32).to_le_bytes());
    archive[26..28].copy_from_slice(&(name.len() as u16).to_le_bytes());
    archive.extend_from_slice(name); archive.extend_from_slice(&table);
    std::fs::create_dir_all(f.api.game_dir.join("package")).unwrap();
    std::fs::write(f.api.game_dir.join("package/data002.ypk"), archive).unwrap();
    assert_eq!(f.api.account_countries(Actor::Host { by: "host" }).unwrap().codes, ["FR", "JP"]);
    let request = AccountEditRequest::SetCountry { user_id: f.user, country: Some("jp".into()) };
    let first = f.api.edit_account(&request, ID, Actor::Host { by: "host" }, "server").unwrap();
    assert_eq!(f.api.account_detail(f.user, Actor::Host { by: "host" }).unwrap().country.as_deref(), Some("JP"));
    assert_eq!(f.api.accounts.lock().unwrap().player(f.player).unwrap().country, "JP");
    assert_eq!(f.api.edit_account(&AccountEditRequest::SetCountry { user_id: f.user, country: Some("ZZ".into()) }, "unknown-country-00001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
    std::fs::remove_file(f.api.game_dir.join("package/data002.ypk")).unwrap();
    assert_eq!(f.api.edit_account(&request, ID, Actor::Host { by: "host" }, "server").unwrap(), first, "replay cannot depend on assets still being installed");
    assert_eq!(f.api.edit_account(&AccountEditRequest::SetCountry { user_id: f.user, country: Some("bad".into()) }, "invalid-country-00001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
}

#[test]
fn deleting_a_password_account_scrubs_all_numeric_profile_audits_but_keeps_safe_retry_receipts() {
    use yo3_server::logins::{Login, Registration};
    let f = Fixture::new("delete-privacy");
    let mut d = f.api.accounts.lock().unwrap();
    let user = d.register(Login::new("TargetLogin", "private_hash".into(), "private_recovery".into()), None, Registration::Open).unwrap();
    d.add_player_to(user, "OriginalAlias").unwrap();
    let player = d.player_id_named("OriginalAlias").unwrap(); drop(d);
    let first = AccountEditRequest::RenamePlayer { user_id: user, player_id: player, name: "MiddleAlias".into() };
    let receipt = f.api.edit_account(&first, ID, Actor::Host { by: "host" }, "dashboard").unwrap();
    f.api.edit_account(&AccountEditRequest::RenamePlayer { user_id: user, player_id: player, name: "FinalAlias".into() }, "profile-final-00000001", Actor::Host { by: "host" }, "dashboard").unwrap();
    f.api.apply(&Op::DeleteAccount { who: "TargetLogin".into() }, "host").unwrap();
    let rows = f.api.audit(100).unwrap();
    assert_eq!(rows.len(), 1, "only the deletion audit may remain: {}", serde_json::to_string(&rows).unwrap());
    assert_eq!(rows[0].op, "delete-account");
    assert_eq!(f.api.edit_account(&first, ID, Actor::Host { by: "host" }, "dashboard").unwrap(), receipt);
    assert!(f.api.accounts.lock().unwrap().account_of(user).is_none());
}
