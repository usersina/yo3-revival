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
    /// With `add-account` and `key`: the address friends reach the server at, written into the `yo3.ini` lines.
    #[arg(long, global = true)]
    advertise: Option<String>,
    #[arg(long, default_value_t = 5730, global = true)]
    gate_port: u16,
    #[arg(long, default_value_t = 1213)]
    front_port: u16,
    /// Duel connections (the original's port is unknown).
    #[arg(long, default_value_t = 1214)]
    duel_port: u16,
    #[arg(long, default_value_t = 8080, global = true)]
    http_port: u16,
    /// Turn the self-service join page (`http://HOST:8080/join`) off, for a server reachable from the internet: its
    /// keys travel in plain HTTP. Hosts then make accounts with `add-account`.
    #[arg(long)]
    no_join: bool,
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
    /// Every player's extra deck in a duel, as CARD_Prop rows (to test an extra deck summon).
    #[arg(long, value_delimiter = ',')]
    duel_extra: Vec<u16>,
    /// Every duel between players' turn and total clocks in seconds, 0 for off: `--duel-time 0,0` (to test slowly).
    #[arg(long, value_delimiter = ',')]
    duel_time: Option<Vec<u32>>,
    /// Check every duel's deck against this regulation (an id) whatever the duel names, so a deck the client let through
    /// can be refused (to test the server's own check; a stock client checks the duel's regulation itself first).
    #[arg(long)]
    duel_regulation: Option<u32>,
    /// The banlist players log in with, for Auto Duel and NPC duels: a regulation id or name, e.g. "Season 16" or 1
    /// (the Beta list). Default: "unlimited" (no banned or limited cards). Hosted duels use the host's choice.
    #[arg(long)]
    regulation: Option<String>,
}

#[derive(Subcommand)]
enum Admin {
    /// List every account: its address (or none), key and players.
    Accounts,
    /// Make an account found by a secret key, not an address: NAME is its first player (the starting players when
    /// omitted), --also adds up to three more. Prints the key and the `yo3.ini` lines to send the friend (the host in
    /// them is `--advertise`, or the placeholder HOST).
    AddAccount {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        also: Vec<String>,
    },
    /// Print the key of an existing account, ADDRESS or one of its PLAYER names (making a key if it has none; the
    /// account keeps its address too), with the `yo3.ini` lines.
    Key { who: String },
    /// Give an account (its ADDRESS or one of its PLAYER names) a player named NAME; a new address gets an account
    /// with only that player.
    AddPlayer { who: String, name: String },
    /// Rename a player.
    Rename { old: String, new: String },
    /// Move the account at OLD to NEW, for a friend whose address changed (an account with a key needs no move: the
    /// key follows the friend): players, cards, BP, decks and records go along. An account already at NEW (made
    /// when the friend first connected from there) is only discarded with --replace.
    MoveAccount {
        old: IpAddr,
        new: IpAddr,
        #[arg(long)]
        replace: bool,
    },
    /// Set the country flag the account at WHO (an address or a player's name) shows to others (a code from the
    /// client's table, such as FR or JP: `countries` lists them); "auto" goes back to the one the client's language
    /// implies.
    SetCountry { who: String, code: String },
    /// List the country codes the client has flags for.
    Countries,
}

/// The `yo3.ini` a friend with `key` pastes: every line points at the host (`HOST` when not given).
fn ini_lines(a: &Args, key: &str) -> String {
    let host = a.advertise.as_deref().unwrap_or("HOST");
    format!(
        "GateServerAddress={host}\nGateServerPort={}\nPatchUpdateUrl=http://{host}:{}/yo3/{key}/\nPatchDataUrl=http://{host}:{}/ygoda/patch/\nInfoURL=http://{host}:{}/app/",
        a.gate_port, a.http_port, a.http_port, a.http_port
    )
}

fn print_key(a: &Args, key: &str) {
    println!("key: {key}\n\nThe friend's yo3.ini (the [Yo3] section; the key is the secret in PatchUpdateUrl, keep it to them):\n\n{}", ini_lines(a, key));
    if a.advertise.is_none() {
        println!("\nReplace HOST with the address friends reach the server at (or pass --advertise ADDRESS).");
    }
}

/// Runs an admin command against the accounts file.
fn admin(a: &Args, command: Admin, game_dir: PathBuf) -> anyhow::Result<()> {
    let mut d = Directory::open(a.players.clone(), &a.data_dir.join("accounts.json"))?;
    let err = anyhow::Error::msg;
    match command {
        Admin::Accounts => {
            for (addr, key, acc) in d.entries() {
                let names: Vec<_> = acc.players.iter().map(|p| format!("{} (#{})", p.name, p.player_id)).collect();
                let country = if acc.country.is_empty() { "auto".to_string() } else { acc.country.clone() };
                let addr = addr.map_or("no address".to_string(), |a| a.to_string());
                println!("{addr}\tuser {}\tkey {}\tcountry {country}\t{}", acc.user_id, key.unwrap_or("none"), names.join(", "));
            }
        }
        Admin::AddAccount { name, also } => {
            let names: Vec<String> = name.into_iter().chain(also).collect();
            let key = d.add_account(&names).map_err(err)?;
            print_key(a, &key);
        }
        Admin::Key { who } => {
            let key = d.key_of(&who).map_err(err)?;
            print_key(a, &key);
        }
        Admin::AddPlayer { who, name } => match who.parse::<IpAddr>() {
            Ok(address) => d.add_player(address, &name).map_err(err)?,
            Err(_) => {
                let user = d.user_of(&who).ok_or_else(|| err(format!("no player named {who:?}")))?;
                d.add_player_to(user, &name).map_err(err)?
            }
        },
        Admin::Rename { old, new } => d.rename(&old, &new).map_err(err)?,
        Admin::MoveAccount { old, new, replace } => d.move_account(old, new, replace).map_err(err)?,
        Admin::SetCountry { who, code } => {
            let code = if code.eq_ignore_ascii_case("auto") { String::new() } else { code.to_ascii_uppercase() };
            let known = yo3_data::countries(&game_dir)?;
            if !code.is_empty() && !known.contains(&code) {
                anyhow::bail!("{code} is not in the client's country table; `countries` lists them");
            }
            let user = d.user_of(&who).ok_or_else(|| err(format!("no account at or with a player named {who:?}")))?;
            d.set_country_of(user, &code).map_err(err)?
        }
        Admin::Countries => println!("{}", yo3_data::countries(&game_dir)?.join(" ")),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_target(false)
        .init();
    let mut a = Args::parse();
    if let Some(command) = a.admin.take() {
        let game_dir = a.game_dir.clone().unwrap_or_else(yo3_data::default_game_dir);
        return admin(&a, command, game_dir);
    }
    let cfg = Config {
        bind: a.bind,
        advertise: a.advertise,
        gate_port: a.gate_port,
        front_port: a.front_port,
        duel_port: a.duel_port,
        http_port: a.http_port,
        join: !a.no_join,
        game_dir: a.game_dir.unwrap_or_else(yo3_data::default_game_dir),
        log_dir: a.log_dir,
        players: a.players,
        duel_lp: a.duel_lp,
        duel_regulation: a.duel_regulation,
        duel_deck: a.duel_deck,
        duel_extra: a.duel_extra,
        duel_time: a.duel_time.map(|t| (t.first().copied().unwrap_or(0), t.get(1).copied().unwrap_or(0))),
        data_dir: Some(a.data_dir),
        regulation: a.regulation,
        idle_timeout: yo3_server::IDLE_TIMEOUT,
        engine_problem: None,
    };
    start(cfg).await?.wait().await;
    Ok(())
}
