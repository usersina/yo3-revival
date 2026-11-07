use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;
use yo3_server::{Config, start};

/// Yu-Gi-Oh! ONLINE 3 private server (gate, front, duel and patch HTTP).
#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    bind: IpAddr,
    /// Front-server address handed to clients by the gate.
    #[arg(long, default_value = "127.0.0.1")]
    advertise: String,
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
    /// Players each new account starts with, comma separated (at most 4; one account per address).
    #[arg(long, value_delimiter = ',', default_value = "test")]
    players: Vec<String>,
    /// Each player's LP at the start of a duel (lower it to reach a duel's end quickly when testing).
    #[arg(long, default_value_t = 8000)]
    duel_lp: u32,
    /// Every player's main deck in a duel, as CARD_Prop rows repeated to 40 cards (to test a card's prompts).
    #[arg(long, value_delimiter = ',')]
    duel_deck: Vec<u16>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_target(false)
        .init();
    let a = Args::parse();
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
    };
    start(cfg).await?.wait().await;
    Ok(())
}
