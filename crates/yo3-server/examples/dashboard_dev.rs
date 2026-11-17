//! Disposable loopback server for the connected dashboard and its browser tests. Never opens the host's data folder.
use std::sync::{Arc, Mutex};
use yo3_server::{accounts::Directory, logins::{self, Login, Registration, Role}, api::Api, front::{Hub, World}, gifts::{Card, Gifts, Recipe}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = std::env::args().nth(1).unwrap_or_else(|| "18080".into()).parse()?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
    let scratch = std::env::temp_dir().join(format!("yo3-dashboard-dev-{}-{stamp}", std::process::id()));
    let dir = scratch.clone();
    yo3_server::store::initialize(&dir)?;
    let mut accounts = Directory::open(vec![], &dir.join("accounts.json"))?;
    let password = logins::hash_password("DashboardDemo123!".into()).await?;
    for (user, role) in [("TestAdmin", Role::Admin), ("TestModerator", Role::Moderator), ("TestPlayer", Role::Player), ("TestRoleTarget", Role::Player), ("TestRoleAdmin", Role::Admin), ("TestResetTarget", Role::Player), ("TestModTarget", Role::Player), ("TestModAdmin", Role::Admin), ("TestDeleteTarget", Role::Player), ("TestDeleteAdmin", Role::Admin)] {
        let login = Login::new(user, password.clone(), logins::digest(&logins::new_code()));
        let id = accounts.register(login, None, Registration::Open).map_err(anyhow::Error::msg)?;
        accounts.set_role(user, role).map_err(anyhow::Error::msg)?;
        accounts.add_player_to(id, user).map_err(anyhow::Error::msg)?;
    }
    let accounts = Arc::new(Mutex::new(accounts));
    let mut hub = Hub::with_records(&dir.join("records.json"));
    hub.set_directory(accounts.clone()); hub.keep_mail(&dir.join("mail.json"));
    hub.keep_social(&dir.join("friends.json"));hub.keep_teams(&dir.join("teams.json"));
    hub.keep_events(&dir.join("events.json"));hub.keep_rentals(&dir.join("rentals.json"));
    let mut world = World::new(vec![]); world.card_rows = 1000;
    let mut api = Api::new(accounts, Arc::new(yo3_server::settings::Live::open(Some(&dir.join("settings.json")), vec![])?), Some(dir.clone()),
        scratch.join("no-game-files-needed"), "dashboard-dev").with_gifts(Gifts {
            cards: vec![Card { row: 816, cid: 4007, art: 1, name: "Blue-Eyes White Dragon".into() }, Card { row: 817, cid: 4007, art: 2, name: "Blue-Eyes White Dragon".into() }],
            recipes: vec![Recipe { id: 7, title: "Example deck recipe".into(), item: 769, cards: vec![816,816,817] }],
        });
    api.sessions = Some(Arc::new(Mutex::new(yo3_server::auth::Sessions::default())));
    api.hub = Some(Arc::new(Mutex::new(hub))); api.world = Some(Arc::new(world));
    api.titles = Some(Arc::new(yo3_server::titles::TitleData { by_name: [("GAME MASTER".into(), 300)].into(), ..Default::default() }));
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!("Disposable dashboard API: http://{} (TestAdmin / DashboardDemo123!)", listener.local_addr()?);
    println!("Sample gift catalogue; real router, operations and SQLite. No game listeners.");
    let router = yo3_server::dashboard::router(yo3_server::dashboard::Dashboard { api: Arc::new(api), panel: Arc::new(yo3_server::panel::Panel::new()) });
    axum::serve(listener, router.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; }).await?;
    yo3_server::store::close(&dir);
    let _ = std::fs::remove_dir_all(scratch);
    Ok(())
}
