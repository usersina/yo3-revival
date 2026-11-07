use std::net::IpAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use yo3_server::accounts::Directory;
use yo3_server::{Config, start};

/// Yu-Gi-Oh! ONLINE 3 private server (gate, front, duel and patch HTTP).
#[derive(Parser)]
struct Args {
    /// An admin command on the data directory instead of running the server (stop the server first: it rewrites
    /// the accounts file when a new friend connects).
    #[command(subcommand)]
    admin: Option<Admin>,
    /// Where accounts (accounts.json) and duel records (records.json) are kept; each start backs them up into
    /// backups/.
    #[arg(long, default_value = "data", global = true)]
    data_dir: PathBuf,
    #[arg(long, default_value = "127.0.0.1")]
    bind: IpAddr,
    /// The address clients are sent to for the front and duel servers (default: the address each client reached;
    /// set it when clients reach the server through a forwarded port or a different name).
    #[arg(long)]
    advertise: Option<String>,
    #[arg(long, default_value_t = 5730)]
    gate_port: u16,
    #[arg(long, default_value_t = 1213)]
    front_port: u16,
    /// Duel connections (the original's port is unknown).
    #[arg(long, default_value_t = 1214)]
    duel_port: u16,
    #[arg(long, default_value_t = 8080)]
    http_port: u16,
    /// Game install (for the patch index and the client's own data files).
    #[arg(long)]
    game_dir: Option<PathBuf>,
    #[arg(long, default_value = "logs")]
    log_dir: PathBuf,
    /// Players each new account starts with, comma separated (at most 4; one account per address); a name already
    /// in use gets a number (Duelist2...).
    #[arg(long, value_delimiter = ',', default_value = "Duelist", global = true)]
    players: Vec<String>,
    /// Each player's LP at the start of a duel (lower it to reach a duel's end quickly when testing).
    #[arg(long, default_value_t = 8000)]
    duel_lp: u32,
    /// Every player's main deck in a duel, as CARD_Prop rows repeated to 40 cards (to test a card's prompts).
    #[arg(long, value_delimiter = ',')]
    duel_deck: Vec<u16>,
    /// The banlist players log in with, for Auto Duel and NPC duels: a regulation id or name, e.g. "unlimited" (no
    /// banned or limited cards) or "Season 16". Hosted duels use the host's choice.
    #[arg(long)]
    regulation: Option<String>,
}

#[derive(Subcommand)]
enum Admin {
    /// List every account: its address and players.
    Accounts,
    /// Give the account at ADDRESS (a friend's LAN or VPN address) a player named NAME; a new address gets an account
    /// with only that player.
    AddPlayer { address: IpAddr, name: String },
    /// Rename a player.
    Rename { old: String, new: String },
}

/// Runs an admin command against the accounts file.
fn admin(command: Admin, data_dir: &std::path::Path, players: Vec<String>) -> anyhow::Result<()> {
    let mut d = Directory::open(players, &data_dir.join("accounts.json"))?;
    match command {
        Admin::Accounts => {
            for (addr, a) in d.accounts() {
                let names: Vec<_> = a.players.iter().map(|p| format!("{} (#{})", p.name, p.player_id)).collect();
                println!("{addr}\tuser {}\t{}", a.user_id, names.join(", "));
            }
        }
        Admin::AddPlayer { address, name } => d.add_player(address, &name).map_err(anyhow::Error::msg)?,
        Admin::Rename { old, new } => d.rename(&old, &new).map_err(anyhow::Error::msg)?,
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_target(false)
        .init();
    let a = Args::parse();
    if let Some(command) = a.admin {
        return admin(command, &a.data_dir, a.players);
    }
    let cfg = Config {
        bind: a.bind,
        advertise: a.advertise,
        gate_port: a.gate_port,
        front_port: a.front_port,
        duel_port: a.duel_port,
        http_port: a.http_port,
        game_dir: a.game_dir.unwrap_or_else(yo3_data::default_game_dir),
        log_dir: a.log_dir,
        players: a.players,
        duel_lp: a.duel_lp,
        duel_deck: a.duel_deck,
        data_dir: Some(a.data_dir),
        regulation: a.regulation,
    };
    start(cfg).await?.wait().await;
    Ok(())
}
