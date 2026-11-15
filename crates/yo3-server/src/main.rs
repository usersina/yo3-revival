use std::net::IpAddr;
use std::path::PathBuf;

use clap::{CommandFactory, FromArgMatches};
use clap::parser::ValueSource;
use yo3_server::cli::{Args, Admin};
use yo3_server::accounts::Directory;
use yo3_server::admin::{self as live, Op};
use yo3_server::{Config, start};

/// The private launcher profile (`HOST` is a placeholder when no address was supplied).
fn profile(a: &Args, key: &str) -> String {
    let host = a.advertise.as_deref().unwrap_or("HOST");
    let address = match host.parse::<IpAddr>() { Ok(IpAddr::V6(_)) => format!("[{host}]"), _ => host.to_string() };
    serde_json::json!({ "server": format!("{address}:{}", a.auth_port), "server_name": host,
        "certificate": "server.crt", "key": key }).to_string()
}

fn print_key(a: &Args, key: &str) {
    println!("Private launcher profile (save as client.json; keep it to this friend):\n{}", profile(a, key));
    if a.advertise.is_none() {
        println!("\nReplace HOST with the address friends reach the server at (or pass --advertise ADDRESS).");
    }
}

/// The settings of a stopped server's data folder, with the game's banlist names when its data loads (a banlist by name
/// needs them; by id it doesn't).
fn offline_settings(data_dir: &std::path::Path, game_dir: &std::path::Path) -> anyhow::Result<yo3_server::settings::Live> {
    let banlists = yo3_data::GameData::load(game_dir).map(|data| {
        data.regulations.iter().map(|r| (r.id, data.text(r.name_id).unwrap_or_default().to_string())).collect()
    }).unwrap_or_default();
    Ok(yo3_server::settings::Live::open(Some(&data_dir.join("settings.json")), banlists)?)
}

/// Fills what the command line left at its default from the TOML file (a flag wins over the file); the file's engine
/// path, for the server to find (`YO3_ENGINE`).
fn apply_file(a: &mut Args, matches: &clap::ArgMatches) -> anyhow::Result<Option<std::path::PathBuf>> {
    let Some((_, file)) = yo3_server::config::FileConfig::find(a.config.as_deref())? else { return Ok(None) };
    let unset = |id: &str| matches.value_source(id) != Some(ValueSource::CommandLine)
        && matches.subcommand().and_then(|(_, sub)| sub.value_source(id)) != Some(ValueSource::CommandLine);
    if let Some(v) = file.bind && unset("bind") { a.bind = v }
    if let Some(v) = file.auth_port && unset("auth_port") { a.auth_port = v }
    if let Some(v) = file.http_port && unset("http_port") { a.http_port = v }
    if let Some(v) = file.advertise && unset("advertise") { a.advertise = Some(v) }
    if let Some(v) = file.game_dir && unset("game_dir") { a.game_dir = Some(v) }
    if let Some(v) = file.data_dir && unset("data_dir") { a.data_dir = v }
    if let Some(v) = file.log_dir && unset("log_dir") { a.log_dir = v }
    if let Some(v) = file.auth_cert && unset("auth_cert") { a.auth_cert = Some(v) }
    if let Some(v) = file.auth_key && unset("auth_key") { a.auth_key = Some(v) }
    Ok(file.engine)
}

/// Runs an admin command against persisted accounts, or for the BP commands, the running server.
fn admin(a: &Args, command: Admin, game_dir: PathBuf) -> anyhow::Result<()> {
    // Listings only read; the BP commands go to a running server (admin.rs) when there is one. Everything else owns
    // the data folder while it runs, as a server does (store::lock).
    let read_only = matches!(command, Admin::Accounts | Admin::Countries | Admin::Codes | Admin::Titles | Admin::Backups | Admin::Invites | Admin::Settings);
    let live_op = matches!(command, Admin::GiveBp { .. } | Admin::AddCode { .. } | Admin::RemoveCode { .. } | Admin::Invite { .. }
        | Admin::ResetCode { .. } | Admin::SetRole { .. } | Admin::AddUser { .. } | Admin::Set { .. } | Admin::Reset { .. })
        && live::running(&a.data_dir).is_some();
    let _lock = if read_only || live_op { None } else { Some(yo3_server::store::lock(&a.data_dir)?) };
    // Restoring comes before opening the database: the one being replaced may be the damaged one.
    match &command {
        Admin::Restore { which } => {
            let from = yo3_server::store::restore(&a.data_dir, which)?;
            println!("Restored the backup from {}.", from.file_name().unwrap_or_default().to_string_lossy());
            return Ok(());
        }
        Admin::Backups => {
            for backup in yo3_server::store::backups(&a.data_dir) { println!("{}", backup.file_name().unwrap_or_default().to_string_lossy()) }
            return Ok(());
        }
        _ => {}
    }
    yo3_server::store::initialize(&a.data_dir)?;
    if let Admin::InitAuth { names } = command {
        let tls = yo3_server::auth::initialize(&a.data_dir, names)?;
        println!("Public certificate: {}\nPrivate server key: {}", tls.certificate.display(), tls.private_key.display());
        return Ok(());
    }
    let mut handed = None;
    let op = match &command {
        Admin::GiveBp { who, amount } => Some(Op::GiveBp { who: who.clone(), amount: *amount }),
        Admin::AddCode { code, bp, uses } => Some(Op::AddCode { code: code.clone(), bp: *bp, uses: *uses }),
        Admin::RemoveCode { code } => Some(Op::RemoveCode { code: code.clone() }),
        Admin::Codes => Some(Op::Codes),
        Admin::Invite { uses, days } => Some(Op::Invite { uses: (*uses > 0).then_some(*uses), days: *days }),
        Admin::Invites => Some(Op::Invites),
        Admin::ResetCode { user } => Some(Op::ResetCode { user: user.clone() }),
        Admin::SetRole { user, role } => Some(Op::SetRole { user: user.clone(), role: role.clone() }),
        Admin::Settings => Some(Op::Settings),
        Admin::Set { name, value } => Some(Op::Set { name: name.clone(), value: value.clone() }),
        Admin::Reset { name } => Some(Op::Reset { name: name.clone() }),
        Admin::AddUser { user } => {
            let password = yo3_server::logins::new_password();
            let hash = yo3_server::logins::hash_password_now(&password)?;
            handed = Some(format!("User ID: {user}\nPassword: {password} (they change it in the launcher)"));
            Some(Op::AddUser { user: user.clone(), hash })
        }
        _ => None,
    };
    let server = live::running(&a.data_dir);
    if let Some(op) = op {
        let done = match &server {
            Some(server) => live::send(server, &op),
            None if matches!(op, Op::Settings | Op::Set { .. } | Op::Reset { .. }) => {
                live::apply_settings(&offline_settings(&a.data_dir, &game_dir)?, &op).expect("a settings operation")
            }
            None => live::apply(&mut Directory::open(a.players.clone(), &a.data_dir.join("accounts.json"))?, &op),
        };
        let done = done.map_err(anyhow::Error::msg)?;
        if let Some(handed) = handed { println!("{handed}") }
        println!("{done}");
        return Ok(());
    }
    // The rest edit the accounts file, which a running server would overwrite.
    if server.is_some() && !matches!(command, Admin::Accounts | Admin::Countries) {
        anyhow::bail!("the server is running on this data directory: stop it first, or its next save overwrites this change");
    }
    let mut d = Directory::open(a.players.clone(), &a.data_dir.join("accounts.json"))?;
    let err = anyhow::Error::msg;
    match command {
        Admin::InitAuth { .. } => unreachable!(),
        Admin::Profile { who, output } => {
            use std::io::Write;
            let key = d.key_of(&who).map_err(err)?;
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)] {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options.open(output)?.write_all(profile(a, &key).as_bytes())?;
        }
        Admin::Accounts => {
            for (login, key, acc) in d.entries() {
                let names: Vec<_> = acc.players.iter().map(|p| format!("{} (#{})", p.name, p.player_id)).collect();
                let country = if acc.country.is_empty() { "auto".to_string() } else { acc.country.clone() };
                // A key is a password: the listing says an account has one, `key WHO` prints it.
                let found_by = match (login, key) { (Some(user), _) => format!("User ID {user}"), (None, Some(_)) => "key".into(), (None, None) => "-".into() };
                let role = d.login_of(login.unwrap_or_default()).map(|(_, l)| format!("\t{:?}", l.role).to_lowercase()).unwrap_or_default();
                println!("user {}\t{found_by}{role}\tcountry {country}\t{}", acc.user_id, names.join(", "));
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
        Admin::AddPlayer { who, name } => {
            let user = d.user_of(&who).ok_or_else(|| err(format!("no account with the User ID or a player named {who:?}")))?;
            d.add_player_to(user, &name).map_err(err)?
        }
        Admin::Rename { old, new } => d.rename(&old, &new).map_err(err)?,
        Admin::SetCountry { who, code } => {
            let code = if code.eq_ignore_ascii_case("auto") { String::new() } else { code.to_ascii_uppercase() };
            let known = yo3_data::countries(&game_dir)?;
            if !code.is_empty() && !known.contains(&code) {
                anyhow::bail!("{code} is not in the client's country table; `countries` lists them");
            }
            let user = d.user_of(&who).ok_or_else(|| err(format!("no account with the User ID or a player named {who:?}")))?;
            d.set_country_of(user, &code).map_err(err)?
        }
        Admin::Countries => println!("{}", yo3_data::countries(&game_dir)?.join(" ")),
        Admin::Titles => {
            let titles = yo3_server::titles::TitleData::from_game(&yo3_data::GameData::load(&game_dir)?);
            let mut names: Vec<_> = titles.by_name.keys().cloned().collect();
            names.sort();
            println!("{}", names.join("\n"));
        }
        Admin::GrantTitle { player, title } => {
            let titles = yo3_server::titles::TitleData::from_game(&yo3_data::GameData::load(&game_dir)?);
            let id = yo3_server::titles::by_name(&titles, &title).ok_or_else(|| err(format!("no title named {title:?}; `titles` lists them")))?;
            let player_id = d.player_id_named(&player).ok_or_else(|| err(format!("no player named {player:?}")))?;
            d.update_player(player_id, |p| { p.items.insert(id); }).map_err(err)?;
            println!("{player} has the title {}", titles.name(id).unwrap_or(&title));
        }
        Admin::GiveBp { .. } | Admin::AddCode { .. } | Admin::RemoveCode { .. } | Admin::Codes | Admin::Restore { .. } | Admin::Backups
        | Admin::Invite { .. } | Admin::Invites | Admin::ResetCode { .. } | Admin::SetRole { .. } | Admin::AddUser { .. }
        | Admin::Settings | Admin::Set { .. } | Admin::Reset { .. } => unreachable!("handled above"),
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let matches = Args::command().get_matches();
    let mut a = Args::from_arg_matches(&matches)?;
    let engine = apply_file(&mut a, &matches)?;
    if let Some(engine) = engine {
        // SAFETY: no other thread runs yet (the async runtime starts below); the engine is found through YO3_ENGINE.
        unsafe { std::env::set_var("YO3_ENGINE", engine) }
    }
    tokio::runtime::Builder::new_multi_thread().enable_all().build()?.block_on(run(a))
}

async fn run(mut a: Args) -> anyhow::Result<()> {
    let log = if a.admin.is_none() { Some(yo3_tunnel::diagnostics::Session::start(&a.diagnostics_dir.clone().unwrap_or_else(|| a.log_dir.join("sessions")), "server", a.diagnostic_parent.clone())?) } else { None };
    let writer = log.clone();
    let color = yo3_tunnel::diagnostics::console_color();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_target(false)
        .with_ansi(false)
        .with_writer(move || yo3_tunnel::diagnostics::ConsoleWriter::new(writer.clone()).colored(color))
        .init();
    if let Some(command) = a.admin.take() {
        let game_dir = a.game_dir.clone().unwrap_or_else(yo3_data::default_game_dir);
        return admin(&a, command, game_dir);
    }
    // Held until the server ends: a second server on this folder is refused before it renews or reads anything.
    let _lock = yo3_server::store::lock(&a.data_dir).inspect_err(|e| tracing::error!("Server startup failed: {e}"))?;
    if a.auth_cert.is_none() && a.auth_key.is_none() {
        yo3_server::auth::renew_on_start(&a.data_dir);
    }
    let cfg = Config {
        tls: yo3_server::auth::TlsConfig { port: a.auth_port,
            certificate: a.auth_cert.unwrap_or_else(|| a.data_dir.join("server.crt")),
            private_key: a.auth_key.unwrap_or_else(|| a.data_dir.join("server.key")) },
        bind: a.bind,
        advertise: a.advertise,
        http_port: a.http_port,
        game_dir: a.game_dir.unwrap_or_else(yo3_data::default_game_dir),
        allow_missing_game_data: false,
        log_dir: a.log_dir,
        packet_logging: !a.no_packet_log, diagnostic_id: log.as_ref().map(|l| l.id()),
        players: a.players,
        duel_lp: a.duel_lp,
        duel_regulation: a.duel_regulation,
        duel_deck: a.duel_deck,
        duel_extra: a.duel_extra,
        duel_time: a.duel_time.map(|t| (t.first().copied().unwrap_or(0), t.get(1).copied().unwrap_or(0))),
        data_dir: Some(a.data_dir),
        idle_timeout: yo3_server::IDLE_TIMEOUT,
        engine_problem: None,
        duel_check: a.duel_check,
        settings: vec![],
    };
    let mut running = match start(cfg).await { Ok(running) => running, Err(error) => { tracing::error!("Server startup failed: {error:#}"); return Err(error) } };
    if a.owned {
        // Standard input stays open while the owning launcher lives; end of input means it has gone.
        let owner_gone = async {
            let _ = tokio::task::spawn_blocking(|| { let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink()); }).await;
        };
        running.exit_when_idle_without_owner(owner_gone, std::time::Duration::from_secs(a.owned_idle));
    }
    running.wait().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_ipv6_profile_has_a_bracketed_address_and_an_unbracketed_certificate_name() {
        let args = <Args as clap::Parser>::try_parse_from(["yo3-server", "--advertise", "::1"]).unwrap();
        let p: serde_json::Value = serde_json::from_str(&profile(&args, "private-test-key")).unwrap();
        assert_eq!(p["server"], "[::1]:7443");
        assert_eq!(p["server_name"], "::1");
    }
}
