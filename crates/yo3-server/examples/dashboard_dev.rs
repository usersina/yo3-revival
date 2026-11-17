//! Disposable loopback server for the connected dashboard and its browser tests. Never opens the host's data folder.
use yo3_server::{accounts::Directory, logins::{self, Login, Registration, Role}, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = std::env::args().nth(1).unwrap_or_else(|| "18080".into()).parse()?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
    let scratch = std::env::temp_dir().join(format!("yo3-dashboard-dev-{}-{stamp}", std::process::id()));
    let mut cfg = Config::ephemeral(scratch.clone());
    cfg.dashboard_port = Some(port);
    cfg.game_dir = scratch.join("no-game-files-needed");
    cfg.engine_problem = Some("Dashboard-only development server".into());
    cfg.packet_logging = false;
    cfg.packet_traces = false;
    let dir = cfg.data_dir.clone().unwrap();
    yo3_server::store::initialize(&dir)?;
    let mut accounts = Directory::open(vec![], &dir.join("accounts.json"))?;
    let password = logins::hash_password("DashboardDemo123!".into()).await?;
    for (user, role) in [("TestAdmin", Role::Admin), ("TestModerator", Role::Moderator), ("TestPlayer", Role::Player), ("TestRoleTarget", Role::Player), ("TestRoleAdmin", Role::Admin)] {
        let login = Login::new(user, password.clone(), logins::digest(&logins::new_code()));
        let id = accounts.register(login, None, Registration::Open).map_err(anyhow::Error::msg)?;
        accounts.set_role(user, role).map_err(anyhow::Error::msg)?;
        accounts.add_player_to(id, user).map_err(anyhow::Error::msg)?;
    }
    drop(accounts);
    let running = yo3_server::start(cfg).await?;
    println!("Disposable dashboard API: http://{} (TestAdmin / DashboardDemo123!)", running.dashboard.unwrap());
    println!("Separate local management/assets: http://{}", running.http);
    tokio::signal::ctrl_c().await?;
    running.stopper().send_replace(true);
    running.wait().await;
    yo3_server::store::close(&dir);
    let _ = std::fs::remove_dir_all(scratch);
    Ok(())
}
