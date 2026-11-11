//! Shared CLI definitions: the server parser is also the desktop option catalogue.
use std::net::IpAddr;
use std::path::PathBuf;
use clap::{Parser, Subcommand, CommandFactory};

/// Yu-Gi-Oh! ONLINE 3 private server (gate, front, duel and patch HTTP).
#[derive(Parser)]
#[command(version = yo3_tunnel::BUILD_VERSION)]
pub struct Args {
    /// An admin command on the data directory instead of running the server. The BP ones (give-bp, add-code,
    /// remove-code, codes) reach a running server; the others need it stopped (it rewrites the accounts file).
    #[command(subcommand)]
    pub admin: Option<Admin>,
    /// Where persistent state (SQLite server.db) is kept; each start backs it up into backups/.
    #[arg(long, default_value = "data", global = true, overrides_with = "data_dir")]
    pub data_dir: PathBuf,
    #[arg(long, default_value = "127.0.0.1", overrides_with = "bind")]
    pub bind: IpAddr,
    /// The DNS name or IP friends reach in exported launcher profiles and advertised asset URLs.
    #[arg(long, global = true)]
    pub advertise: Option<String>,
    /// Authenticated launcher connections over TLS.
    #[arg(long, default_value_t = 7443, global = true)]
    pub auth_port: u16,
    #[arg(long, global = true)]
    pub auth_cert: Option<PathBuf>,
    #[arg(long)]
    pub auth_key: Option<PathBuf>,
    #[arg(long, default_value_t = 8080, global = true)]
    pub http_port: u16,
    /// Game install (for the patch index and the client's own data files).
    #[arg(long)]
    pub game_dir: Option<PathBuf>,
    #[arg(long, default_value = "logs", overrides_with = "log_dir")]
    pub log_dir: PathBuf,
    /// Disable raw game packet logging (the desktop defaults to bounded diagnostics).
    #[arg(long)] pub no_packet_log: bool,
    #[arg(long)] pub diagnostics_dir: Option<PathBuf>,
    #[arg(long)] pub diagnostic_parent: Option<String>,
    /// Default players for add-account, comma separated (at most 4); a name already
    /// in use gets a number (Duelist2...).
    #[arg(long, value_delimiter = ',', default_value = "Duelist", global = true)]
    pub players: Vec<String>,
    /// Each player's LP at the start of a duel (lower it to reach a duel's end quickly when testing).
    #[arg(long, default_value_t = 8000)]
    pub duel_lp: u32,
    /// Every player's main deck in a duel, as CARD_Prop rows repeated to 40 cards (to test a card's prompts).
    #[arg(long, value_delimiter = ',')]
    pub duel_deck: Vec<u16>,
    /// Every player's extra deck in a duel, as CARD_Prop rows (to test an extra deck summon).
    #[arg(long, value_delimiter = ',')]
    pub duel_extra: Vec<u16>,
    /// Every duel between players' turn and total clocks in seconds, 0 for off: `--duel-time 0,0` (to test slowly).
    #[arg(long, value_delimiter = ',')]
    pub duel_time: Option<Vec<u32>>,
    /// Check every duel's deck against this regulation (an id) whatever the duel names, so a deck the client let through
    /// can be refused (to test the server's own check; a stock client checks the duel's regulation itself first).
    #[arg(long)]
    pub duel_regulation: Option<u32>,
    /// Check every duel's client view against the rules engine after each step and log an ERROR where they differ (a
    /// testing aid: the protocol monkey's duels run with it; each step costs an extra engine query).
    #[arg(long)]
    pub duel_check: bool,
    /// The banlist players log in with, for Auto Duel and NPC duels: a regulation id or name, e.g. "Season 16" or 1
    /// (the Beta list). Default: "unlimited" (no banned or limited cards). Hosted duels use the host's choice.
    #[arg(long)]
    pub regulation: Option<String>,
}

#[derive(Subcommand)]
pub enum Admin {
    /// Generate the host's TLS identity once. NAME is the DNS name or IP friends connect to; repeat for aliases.
    InitAuth { #[arg(required = true)] names: Vec<String> },
    /// Export an existing account's private launcher profile. Send it and the public server.crt to that friend.
    Profile { who: String, output: PathBuf },
    /// List every account: its address (or none), key and players.
    Accounts,
    /// Make an account found by a secret key, not an address: NAME is its first player (the starting players when
    /// omitted), --also adds up to three more. Prints a private launcher profile (the host in
    /// them is `--advertise`, or the placeholder HOST).
    AddAccount {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        also: Vec<String>,
    },
    /// Print a private launcher profile for an existing account, ADDRESS or one of its PLAYER names (assigning
    /// a key if needed; existing player data stays in place).
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
    /// Give BOOSTERPASS Points to the account at WHO (an address or one of its players' names). Works while the server
    /// runs (on the server's PC).
    GiveBp { who: String, amount: u32 },
    /// Make a BOOSTERPASS code: CODE (16 letters and digits, the game's window) gives BP to each account that enters it
    /// in Main Menu -> BOOSTERPASS, once; --uses N stops it after N accounts. Works while the server runs.
    AddCode {
        code: String,
        bp: u32,
        #[arg(long)]
        uses: Option<u32>,
    },
    /// Remove a BOOSTERPASS code. Works while the server runs.
    RemoveCode { code: String },
    /// List the BOOSTERPASS codes and how often each was used. Works while the server runs.
    Codes,
}

/// Run flags (administrative subcommands are separate actions, not persistent server settings).
#[derive(Clone, Debug)]
pub struct ServerOption { pub name: String, pub help: String, pub default: String, pub toggle: bool }
pub fn server_options() -> Vec<ServerOption> {
    let mut command = Args::command(); command.build();
    command.get_arguments().filter_map(|arg| {
        let name = arg.get_long()?;
        if ["help", "version"].contains(&name) { return None; }
        Some(ServerOption { name: name.into(), help: arg.get_help().map(ToString::to_string).unwrap_or_else(|| name.replace('-', " ")),
            default: arg.get_default_values().iter().map(|v| v.to_string_lossy()).collect::<Vec<_>>().join(","),
            toggle: matches!(arg.get_action(), clap::ArgAction::SetTrue | clap::ArgAction::SetFalse) })
    }).collect()
}
pub fn validate_server_args<'a>(args: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
    let parsed = Args::try_parse_from(std::iter::once("yo3-server").chain(args)).map_err(|e| e.to_string())?;
    if parsed.admin.is_some() { return Err("Administrative commands cannot be saved as server options.".into()); }
    Ok(())
}
