//! Host gifts commit player state, D-Mail, audit and retry receipt before any game push.
mod common;
use std::{path::PathBuf, sync::{Arc, Mutex}};
use yo3_server::{accounts::Directory, api::{Api, Op}, gifts::{Card, Gifts, Recipe}, settings, store, titles::TitleData};
struct Fixture { dir: PathBuf, api: Api, client: common::Client, player: u32 }
impl Fixture {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("yo3-admin-gifts-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir); store::initialize(&dir).unwrap();
        let mut d = Directory::open(vec![], &dir.join("accounts.json")).unwrap(); d.add_account(&["Yugi".into()]).unwrap();
        let user = d.user_of("Yugi").unwrap(); let player = d.player_id_named("Yugi").unwrap();
        let accounts = Arc::new(Mutex::new(d));
        let mut world = yo3_server::front::World::new(vec![]); world.card_rows = 1000;
        let server = common::Server::with_world(world);
        server.hub.lock().unwrap().set_directory(accounts.clone()); server.hub.lock().unwrap().keep_mail(&dir.join("mail.json"));
        let mut client = server.join_saved(&accounts, user); client.pushed();
        let mut api = Api::new(accounts, Arc::new(settings::Live::open(None, vec![]).unwrap()), Some(dir.clone()), Default::default(), "server").with_gifts(Gifts {
            cards: vec![Card { row: 816, cid: 4007, art: 1, name: "Blue-Eyes White Dragon".into() }],
            recipes: vec![Recipe { id: 7, title: "E-HERO EVOLUTION DECK".into(), item: 769, cards: vec![816,816,816] }],
        });
        api.hub = Some(server.hub); api.world = Some(server.world);
        api.titles = Some(Arc::new(TitleData { by_name: [("GAME MASTER".into(), 300)].into(), ..Default::default() }));
        Self { dir, api, client, player }
    }
    fn sql(&self, sql: &str) { rusqlite::Connection::open(self.dir.join("server.db")).unwrap().execute_batch(sql).unwrap(); }
}
impl Drop for Fixture { fn drop(&mut self) { store::close(&self.dir); let _ = std::fs::remove_dir_all(&self.dir); } }
fn op(kind: &str) -> Op { match kind {
    "cards" => Op::GiveCards { player: "Yugi".into(), card: "4007".into(), art: None, count: Some(3) },
    "recipe" => Op::GiveRecipe { player: "Yugi".into(), deck: "7".into(), cards: true },
    _ => Op::GrantTitle { player: "Yugi".into(), title: "GAME MASTER".into() },
} }
#[test]
fn gifts_cannot_report_success_or_notify_players_without_their_audit() {
    for kind in ["cards", "recipe", "title"] {
        let mut f = Fixture::new(kind);
        f.sql("CREATE TRIGGER refuse_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'no audit'); END;");
        assert!(f.api.apply(&op(kind), "host").is_err(), "{kind} reported success without audit");
        for d in [f.api.accounts.lock().unwrap().clone(), Directory::open(vec![], &f.dir.join("accounts.json")).unwrap()] {
            let p = d.player(f.player).unwrap(); assert!(p.cards.is_empty()); assert!(!p.items.contains(&300) && !p.items.contains(&769));
        }
        assert!(f.client.pushed().is_empty());
    }
}

use yo3_server::{api::{Actor, MutationErrorCode, gifts::{GiftRequest, GiftOutcome}}, logins::{Login, Registration, Role}};
const ID: &str = "gift-request-00000001";
fn req(f: &Fixture, kind: &str) -> GiftRequest { match kind {
    "cards" => GiftRequest::Cards { player_id: f.player, row: 816, count: 3 },
    "recipe" => GiftRequest::Recipe { player_id: f.player, deck_id: 7, include_cards: true },
    _ => GiftRequest::Title { player_id: f.player, title_id: 300 },
} }
#[test]
fn all_gift_failures_roll_back_player_mail_receipt_and_notifications_even_after_a_later_save() {
    for kind in ["cards", "recipe", "title"] { for failure in ["save", "mail", "audit", "receipt", "commit"] {
        if kind == "title" && failure == "mail" { continue } // Titles retain their existing no-mail behavior.
        let mut f = Fixture::new(&format!("{kind}-{failure}"));
        let before = f.api.accounts.lock().unwrap().player(f.player).unwrap().clone();
        match failure {
            "save" => store::fail_domain(&f.dir, "accounts.json", true).unwrap(),
            "mail" => store::fail_domain(&f.dir, "mail.json", true).unwrap(),
            "audit" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON audit BEGIN SELECT RAISE(FAIL, 'no audit'); END;"),
            "receipt" => f.sql("CREATE TRIGGER refuse_change BEFORE INSERT ON operation_receipts BEGIN SELECT RAISE(FAIL, 'no receipt'); END;"),
            "commit" => f.sql("CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(parent INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TRIGGER refuse_change AFTER INSERT ON audit BEGIN INSERT INTO child VALUES(-1); END;"),
            _ => unreachable!(),
        }
        assert_eq!(f.api.give_gift(&req(&f, kind), ID, Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::PersistenceFailed);
        for d in [f.api.accounts.lock().unwrap().clone(), Directory::open(vec![], &f.dir.join("accounts.json")).unwrap()] {
            let p = d.player(f.player).unwrap(); assert_eq!(p.cards, before.cards); assert_eq!(p.items, before.items);
        }
        assert!(f.client.pushed().is_empty()); assert!(f.api.audit(100).unwrap().is_empty());
        assert_eq!(common::one(f.client.send(common::req(0x2583))).get("count").and_then(yo3_proto::Value::as_u32), Some(0));
        match failure { "save" => store::fail_domain(&f.dir, "accounts.json", false).unwrap(), "mail" => store::fail_domain(&f.dir, "mail.json", false).unwrap(), _ => f.sql("DROP TRIGGER refuse_change;") }
        let user = f.api.accounts.lock().unwrap().user_of("Yugi").unwrap();
        f.api.accounts.lock().unwrap().set_country_of(user, "FR").unwrap();
        assert_eq!(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap().player(f.player).unwrap().cards, before.cards);
        let result = f.api.give_gift(&req(&f, kind), ID, Actor::Host { by: "host" }, "server").unwrap();
        assert_eq!(result.player_id, f.player); assert_eq!(f.api.audit(100).unwrap().len(), 1);
        assert_eq!(f.client.pushed().iter().any(|m| m.request() == Some(0x2581)), kind != "title");
    } }
}
#[test]
fn concurrent_retries_commit_once_without_repeating_mail_presents_or_trunks_and_survive_restart() {
    for kind in ["cards", "recipe", "title"] {
        let mut f = Fixture::new(&format!("retry-{kind}")); let request = req(&f, kind);
        let results = std::thread::scope(|s| {
            let jobs: Vec<_> = (0..8).map(|_| s.spawn(|| f.api.give_gift(&request, ID, Actor::Host { by: "host" }, "server").unwrap())).collect();
            jobs.into_iter().map(|job| job.join().unwrap()).collect::<Vec<_>>()
        });
        assert!(results.iter().all(|r| r == &results[0])); assert_eq!(f.api.audit(100).unwrap().len(), 1);
        let pushed = f.client.pushed();
        assert_eq!(pushed.iter().filter(|m| m.request() == Some(0x2581)).count(), usize::from(kind != "title"));
        assert_eq!(pushed.iter().filter(|m| m.request() == Some(0x2774)).count(), usize::from(kind != "title"));
        assert_eq!(pushed.iter().filter(|m| m.request() == Some(9000)).count(), usize::from(kind != "title"));
        f.sql("UPDATE audit SET at='2000-01-01T00:00:00Z';"); store::prune_audit(&f.dir, 1).unwrap();
        store::close(&f.dir); store::initialize(&f.dir).unwrap();
        // No game metadata needed to acknowledge a committed receipt.
        let api = Api::new(Arc::new(Mutex::new(Directory::open(vec![], &f.dir.join("accounts.json")).unwrap())), f.api.settings.clone(), Some(f.dir.clone()), Default::default(), "offline");
        assert_eq!(api.give_gift(&request, ID, Actor::Host { by: "another display name" }, "offline").unwrap(), results[0]);
        let receipt: String = rusqlite::Connection::open(f.dir.join("server.db")).unwrap().query_row("SELECT result FROM operation_receipts", [], |r| r.get(0)).unwrap();
        for secret in ["Yugi", "Blue-Eyes", "host"] { assert!(!receipt.contains(secret)); }
        let mail = store::read(&f.dir.join("mail.json")).unwrap();
        assert_eq!(mail.as_ref().and_then(|m| m["waiting"].as_array()).map_or(0, Vec::len), usize::from(kind != "title"));
    }
}
#[test]
fn independent_grants_both_apply_and_card_limits_include_every_copy_of_a_recipe() {
    let f = Fixture::new("limits");
    std::thread::scope(|s| { for id in ["independent-gift-0001", "independent-gift-0002"] {
        let f = &f; s.spawn(move || f.api.give_gift(&req(f,"cards"), id, Actor::Host { by: "host" }, "server").unwrap());
    } });
    assert_eq!(f.api.accounts.lock().unwrap().player(f.player).unwrap().cards[&816], 6);
    f.api.accounts.lock().unwrap().update_player(f.player, |p| { p.cards.insert(816, 253); }).unwrap();
    assert_eq!(f.api.give_gift(&req(&f,"recipe"), ID, Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
    assert!(!f.api.accounts.lock().unwrap().player(f.player).unwrap().items.contains(&769));
    let request = GiftRequest::Cards { player_id:f.player, row:816,count:2 };
    let result = f.api.give_gift(&request, ID, Actor::Host { by: "host" }, "server").unwrap();
    assert_eq!(result.gift, GiftOutcome::Cards { row:816, added:2,total:255 });
    assert_eq!(f.api.give_gift(&req(&f,"cards"), "limit-overflow-000001", Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::InvalidRequest);
    assert_eq!(f.api.give_gift(&req(&f,"title"), ID, Actor::Host { by: "host" }, "server").unwrap_err().code, MutationErrorCode::RequestConflict);
}
#[test]
fn only_current_admins_can_give_gifts_or_replay_receipts_and_catalogues_are_bounded() {
    let f = Fixture::new("permissions");
    let admin = f.api.accounts.lock().unwrap().register(Login::new("Admin", "HASH".into(), "RECOVERY".into()), None, Registration::Open).unwrap();
    f.api.accounts.lock().unwrap().set_role("Admin", Role::Admin).unwrap();
    let actor = Actor::Staff { user_id:admin };
    f.api.give_gift(&req(&f,"cards"), ID, actor, "panel").unwrap();
    let choices = f.api.gift_choices("blue-eyes", actor).unwrap(); assert_eq!(choices.cards.len(),1); assert!(choices.recipes.is_empty());
    assert_eq!(f.api.gift_choices("GAME MASTER", actor).unwrap().titles[0].title_id,300);
    assert!(f.api.gift_choices(&"a".repeat(81), actor).is_err());
    f.api.accounts.lock().unwrap().set_role("Admin", Role::Moderator).unwrap();
    assert_eq!(f.api.give_gift(&req(&f,"cards"), ID, actor, "panel").unwrap_err().code, MutationErrorCode::Forbidden);
    assert_eq!(f.api.gift_choices("", actor).unwrap_err().code, MutationErrorCode::Forbidden);
    let base = f.api.accounts.clone();
    let cards = (1..=80).map(|row| Card { row, cid: row, art:1, name: format!("Card {row:03}") }).collect();
    let mut api = Api::new(base, f.api.settings.clone(), None, Default::default(), "server").with_gifts(Gifts { cards,recipes:vec![] });
    api.titles = Some(Arc::new(TitleData::default()));
    let choices = api.gift_choices("", Actor::Host { by:"host" }).unwrap(); assert_eq!(choices.cards.len(),50); assert!(choices.more_cards);
}

#[test]
fn real_cli_retries_acknowledge_the_same_host_receipt_without_game_files_or_repeating_grants() {
    for kind in ["cards","recipe","title"] {
        let f = Fixture::new(&format!("cli-{kind}"));
        f.api.apply_request(&op(kind), "Host label one", "server", None, ID).unwrap();
        let args = match kind { "cards" => vec!["give-cards","Yugi","4007","--count","3"], "recipe" => vec!["give-recipe","Yugi","7","--cards"], _ => vec!["grant-title","Yugi","GAME MASTER"] };
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_yo3-server")).args(["--data-dir",f.dir.to_str().unwrap(),"--by","Another host label"]).args(args).args(["--request-id",ID]).output().unwrap();
        assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
        assert!(!output.stdout.is_empty());
        assert_eq!(f.api.audit(100).unwrap().len(),1);
        let p = Directory::open(vec![], &f.dir.join("accounts.json")).unwrap().player(f.player).unwrap().clone();
        if kind != "title" {assert_eq!(p.cards[&816],3)} else {assert!(p.items.contains(&300))}
    }
}
#[test]
fn renamed_player_retries_resolve_receipts_first_and_duplicate_recipes_and_titles_are_explicit() {
    let f = Fixture::new("rename"); let operation = op("cards");
    let first = f.api.apply_request(&operation,"host","server",None,ID).unwrap();
    f.api.accounts.lock().unwrap().rename("Yugi","NewName").unwrap();
    assert_eq!(f.api.apply_request(&operation,"host","server",None,ID).unwrap(),first);
    let recipe = GiftRequest::Recipe {player_id:f.player,deck_id:7,include_cards:false};
    f.api.give_gift(&recipe,"recipe-only-00000001",Actor::Host{by:"host"},"server").unwrap();
    assert_eq!(f.api.give_gift(&recipe,"recipe-only-00000002",Actor::Host{by:"host"},"server").unwrap_err().code,MutationErrorCode::AlreadyExists);
    let title = req(&f,"title");
    f.api.give_gift(&title,"title-once-000000001",Actor::Host{by:"host"},"server").unwrap();
    assert_eq!(f.api.give_gift(&title,"title-twice-00000001",Actor::Host{by:"host"},"server").unwrap().gift,GiftOutcome::Title{title_id:300,already_owned:true});
}
