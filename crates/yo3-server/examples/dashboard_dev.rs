//! Disposable loopback server for the connected dashboard and its browser tests. Never opens the host's data folder.
#[path="support/card_images.rs"]
mod card_images;
use std::sync::{Arc, Mutex};
use yo3_server::{accounts::Directory, logins::{self, Login, Registration, Role}, api::Api, front::{Hub, World}, gifts::{Card, Gifts, Recipe}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_ansi(false).with_writer(||yo3_server::recent::Tee::new(std::io::stdout())).try_init().map_err(|e|anyhow::anyhow!("{e}"))?;
    let port = std::env::args().nth(1).unwrap_or_else(|| "18080".into()).parse()?;
    // `optional` (the default here) keeps the existing browser checks' password-only staff; `required` is the real default.
    let policy = match std::env::args().nth(2).as_deref() { Some("required") => yo3_server::staff_mfa::StaffMfaPolicy::Required, _ => yo3_server::staff_mfa::StaffMfaPolicy::Optional };
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
    let scratch = std::env::temp_dir().join(format!("yo3-dashboard-dev-{}-{stamp}", std::process::id()));
    let dir = scratch.clone();
    yo3_server::store::initialize(&dir)?;
    let mut accounts = Directory::open(vec![], &dir.join("accounts.json"))?;
    // Browser checks that sign in many times may ask for the cheap test hash (never for a real server).
    if std::env::var_os("YO3_DEV_FAST_HASH").is_some() { logins::use_fast_hashing() }
    let password = logins::hash_password("DashboardDemo123!".into()).await?;
    for (user, role) in [("TestAdmin", Role::Admin), ("TestModerator", Role::Moderator), ("TestPlayer", Role::Player), ("TestRoleTarget", Role::Player), ("TestRoleAdmin", Role::Admin), ("TestResetTarget", Role::Player), ("TestModTarget", Role::Player), ("TestModAdmin", Role::Admin), ("TestDeleteTarget", Role::Player), ("TestDeleteAdmin", Role::Admin), ("TestOverview", Role::Player),
        ("MfaAdmin1", Role::Admin), ("MfaAdmin2", Role::Admin), ("MfaAdmin3", Role::Admin), ("MfaAdmin4", Role::Admin), ("MfaAdmin5", Role::Admin), ("MfaAdmin6", Role::Admin), ("MfaAdmin7", Role::Admin), ("MfaAdmin8", Role::Admin), ("MfaMod1", Role::Moderator)] {
        let login = Login::new(user, password.clone(), logins::digest(&logins::new_code()));
        let id = accounts.register(login, None, Registration::Open).map_err(anyhow::Error::msg)?;
        accounts.set_role(user, role).map_err(anyhow::Error::msg)?;
        accounts.add_player_to(id, user).map_err(anyhow::Error::msg)?;
    }
    card_images::write(&scratch)?;
    let accounts = Arc::new(Mutex::new(accounts));
    let mut hub = Hub::with_records(&dir.join("records.json"));
    // Sample player reports (the game's /report writes the same rows) between the demo accounts' players.
    hub.reports = yo3_server::reports::Reports::at(&dir);
    {
        let d = accounts.lock().unwrap();
        let who = |name: &str| d.player_named(name).map(|(id, _, n)| (id, n)).expect("demo player");
        for (from, to, reason) in [("TestPlayer", "TestRoleTarget", "Spams the chat in the lobby"), ("TestResetTarget", "TestRoleTarget", "Insults in duels"),
            ("TestAdmin", "TestPlayer", "Suspected cheating in an Auto Duel"), ("TestPlayer", "TestResetTarget", "Quit three duels in a row")] {
            let (reporter, target) = (who(from), who(to));
            hub.reports.add((reporter.0, &reporter.1), (target.0, &target.1), reason, 100).expect("sample report");
        }
    }
    // A sample official event with two entrants' standings (the Stadium's events, #18).
    hub.keep_official(&dir);
    {
        use yo3_server::official::{Kind, NewEvent, Prize};
        let now = chrono::Utc::now().timestamp();
        let id = hub.official.create(NewEvent { kind: Kind::Gunslinger, name: "Seeded Gunslinger".into(), start: now - 3600, end: now + 7 * 86_400,
            regulation: None, prize: Prize { participation_bp: 100, threshold: 3, threshold_bp: 400 } }).expect("sample event").id;
        let d = accounts.lock().unwrap();
        for (name, wins) in [("TestPlayer", 3), ("TestResetTarget", 1)] {
            let (player, _, name) = d.player_named(name).expect("demo player");
            hub.official.join(id, player, &name, now);
            for _ in 0..wins { hub.official.record(id, player, &name, 1, now); }
        }
    }
    hub.set_directory(accounts.clone()); hub.keep_mail(&dir.join("mail.json"));
    hub.keep_social(&dir.join("friends.json"));hub.keep_teams(&dir.join("teams.json"));
    hub.keep_events(&dir.join("events.json"));hub.keep_rentals(&dir.join("rentals.json"));
    let mut world = World::new(vec![]); world.card_rows = 1000;
    world.banlists = vec![(3, "Unlimited".into()), (20, "Season 16".into())];
    // Every browser test signs in from 127.0.0.1, so the shared address allowance (production: 10, one back per 3 s) would
    // run dry across a suite. This disposable server widens only that allowance; the per-User-ID backoff after wrong
    // passwords and codes stays at its real default, which is what the checks exercise.
    let live = yo3_server::settings::Live::open(Some(&dir.join("settings.json")), vec![])?;
    live.set("address-burst", "1000").map_err(anyhow::Error::msg)?;
    live.set("address-refill-seconds", "1").map_err(anyhow::Error::msg)?;
    let mut api = Api::new(accounts, Arc::new(live), Some(dir.clone()),
        scratch.clone(), "dashboard-dev").with_gifts(Gifts {
            cards: vec![Card { row: 816, cid: 4007, art: 1, name: "Blue-Eyes White Dragon".into(), description: "Invented browser fixture description.\nSecond line of original game text.".into() }, Card { row: 817, cid: 4007, art: 2, name: "Blue-Eyes White Dragon".into(), description: "Invented browser fixture description.\nSecond line of original game text.".into() }],
            recipes: vec![Recipe { id: 7, title: "Example deck recipe".into(), item: 769, cards: vec![816,816,817] }],
        });
    api.sessions = Some(Arc::new(Mutex::new(yo3_server::auth::Sessions::default())));
    api.hub = Some(Arc::new(Mutex::new(hub))); api.world = Some(Arc::new(world));
    api.titles = Some(Arc::new(yo3_server::titles::TitleData { by_name: [("GAME MASTER".into(), 300)].into(), ..Default::default() }));
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!("Disposable dashboard API: http://{} (TestAdmin / DashboardDemo123!)", listener.local_addr()?);
    println!("Sample gift catalogue; real router, operations and SQLite. No game listeners.");
    tracing::info!("Disposable dashboard API is ready; game listeners are disabled");
    let router = yo3_server::dashboard::router(yo3_server::dashboard::Dashboard { api: Arc::new(api), panel: Arc::new(yo3_server::panel::Panel::with_policy(policy)) });
    axum::serve(listener, router.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; }).await?;
    yo3_server::store::close(&dir);
    let _ = std::fs::remove_dir_all(scratch);
    Ok(())
}
