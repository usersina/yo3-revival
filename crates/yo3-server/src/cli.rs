//! Shared CLI definitions: the server parser is also the desktop option catalogue.
use std::net::IpAddr;
use std::path::PathBuf;
use clap::{Parser, Subcommand, CommandFactory};

/// Yu-Gi-Oh! ONLINE 3 private server (gate, front, duel and patch HTTP).
#[derive(Parser)]
#[command(version = yo3_tunnel::BUILD_VERSION)]
pub struct Args {
    /// An admin command instead of running the server: through the running server's admin API when one runs on the
    /// data directory, else on the directory itself. Every change is in the audit log (`audit`).
    #[command(subcommand)]
    pub admin: Option<Admin>,
    /// The server's TOML file of restart-only settings (listeners, TLS files, folders, the engine); default
    /// server.toml in the current folder when it is there. A flag wins over the file. Live settings are not in it:
    /// `settings` lists them and `set` changes them.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
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
    /// Disable separate raw packet files; bounded session diagnostics still include decoded packet traces.
    #[arg(long)] pub no_packet_log: bool,
    /// Run as the launcher's own server (Singleplayer): the launcher holds standard input open, and once it closes
    /// (the launcher exited or crashed) the server stops after nobody has played for --owned-idle seconds.
    #[arg(long)] pub owned: bool,
    #[arg(long, default_value_t = 120, requires = "owned")] pub owned_idle: u64,
    #[arg(long)] pub diagnostics_dir: Option<PathBuf>,
    #[arg(long)] pub diagnostic_parent: Option<String>,
    /// Default players for add-account, comma separated (at most 4); a name already
    /// in use gets a number (Duelist2...).
    #[arg(long, value_delimiter = ',', default_value = "Duelist", global = true)]
    pub players: Vec<String>,
    /// Testing aid: each player's LP at the start of every duel (lower it to reach a duel's end quickly). Hosts set
    /// the start-lp setting instead.
    #[arg(long)]
    pub duel_lp: Option<u32>,
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
}

#[derive(Subcommand)]
pub enum Admin {
    /// Generate the host's TLS identity once. NAME is the DNS name or IP friends connect to; repeat for aliases.
    InitAuth { #[arg(required = true)] names: Vec<String> },
    /// Export an existing account's private launcher profile. Send it and the public server.crt to that friend.
    Profile { who: String, output: PathBuf },
    /// List every account: its User ID (or that it has a key), role, country and players.
    Accounts,
    /// Make an account found by a secret key, not an address: NAME is its first player (the starting players when
    /// omitted), --also adds up to three more. Prints a private launcher profile (the host in
    /// them is `--advertise`, or the placeholder HOST).
    AddAccount {
        name: Option<String>,
        #[arg(long, value_delimiter = ',')]
        also: Vec<String>,
    },
    /// Print a private launcher profile for an existing account, by its User ID or one of its PLAYER names (assigning
    /// a key if needed; existing player data stays in place).
    Key { who: String },
    /// Give an account (its User ID or one of its PLAYER names) a player named NAME.
    AddPlayer { who: String, name: String },
    /// Rename a player.
    Rename { old: String, new: String },
    /// Set the country flag the account WHO (a User ID or a player's name) shows to others (a code from the
    /// client's table, such as FR or JP: `countries` lists them); "auto" goes back to the one the client's language
    /// implies.
    SetCountry { who: String, code: String },
    /// List the country codes the client has flags for.
    Countries,
    /// Give a player a title by its name, as the game shows it (the staff and promotion ones, GAME MASTER or JUMP
    /// FESTA, or any other); `titles` lists them. They wear it in Avatar Settings.
    GrantTitle { player: String, title: String },
    /// List the titles in the client.
    Titles,
    /// Give BOOSTERPASS Points to the account WHO (a User ID or one of its players' names). Works while the server
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
    /// Make an invite code for the launcher's sign-up (the registration setting's invite mode): --uses N sign-ups (1 when
    /// omitted, 0 for any number), working for --days D (for good when omitted). Works while the server runs.
    Invite {
        #[arg(long, default_value_t = 1)]
        uses: u32,
        #[arg(long)]
        days: Option<u32>,
    },
    /// List the invite codes: whether each still works, who issued it and how many accounts used it. Works while the
    /// server runs.
    Invites,
    /// Make a reset code for a player who lost their password and recovery code (check it is them first): it works once,
    /// for 24 hours, in the launcher's "Forgot password". Works while the server runs.
    ResetCode { user: String },
    /// Make USER a player, moderator or admin. Works while the server runs.
    SetRole { user: String, role: String },
    /// Make a password account with User ID USER and print its first password and recovery code, to hand to that
    /// player (for a server whose registration setting is closed). Works while the server runs.
    AddUser { user: String },
    /// Put a backup back as the server's database (the server must be stopped): WHICH is a folder name under
    /// backups/ (`backups` lists them), or "latest". The database it replaces is kept beside it, renamed.
    Restore { which: String },
    /// List the backups, newest first: one at each start, every hour while running, and at each stop.
    Backups,
    /// List the live settings: each one's value, whether it is the default, and when a change applies. Works while
    /// the server runs.
    Settings,
    /// Change a live setting (`settings` lists them), saved in the database: on a running server it applies without a
    /// restart, at the moment the setting says (at once, at the next login or at the next duel).
    Set { name: String, value: String },
    /// Put a live setting back to its default. Works while the server runs.
    Reset { name: String },
    /// The newest administrative changes: when, who, from where, what, on what, before and after.
    Audit {
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// Stop the running server gracefully: logins close, duels end, everything is saved and backed up.
    Stop,
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
