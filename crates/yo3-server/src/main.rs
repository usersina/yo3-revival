use std::net::IpAddr;
use std::path::PathBuf;

use clap::{CommandFactory, FromArgMatches};
use clap::parser::ValueSource;
use yo3_server::cli::{Args, Admin};
use yo3_server::accounts::Directory;
use std::io::Write;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use yo3_server::admin as live;
use yo3_server::api::{self, Op};
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

/// The settings of a stopped server's data folder, with the game's banlist names when `banlists` (a banlist by name
/// needs them; by id it doesn't).
fn offline_settings(data_dir: &std::path::Path, game_dir: &std::path::Path, banlists: bool) -> anyhow::Result<yo3_server::settings::Live> {
    let banlists = if !banlists { vec![] } else { yo3_data::GameData::load(game_dir).map(|data| {
        data.regulations.iter().map(|r| (r.id, data.text(r.name_id).unwrap_or_default().to_string())).collect()
    }).unwrap_or_default() };
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
    // A hosted server's privacy (#37): no raw packet files and no packet traces in the session log.
    if file.packet_log == Some(false) {
        if unset("no_packet_log") { a.no_packet_log = true }
        if unset("no_packet_trace") { a.no_packet_trace = true }
    }
    Ok(file.engine)
}

/// The API operation a subcommand is (None for the ones that work on the files themselves).
fn operation(command: &Admin) -> anyhow::Result<Option<Op>> {
    Ok(Some(match command {
        Admin::InitAuth { .. } | Admin::Restore { .. } | Admin::Backups => return Ok(None),
        Admin::Profile { who, .. } | Admin::Key { who } => Op::Key { who: who.clone() },
        Admin::Accounts => Op::Accounts,
        Admin::AddAccount { name, also } => Op::CreateAccount { names: name.iter().cloned().chain(also.iter().cloned()).collect() },
        Admin::AddPlayer { who, name } => Op::AddPlayer { who: who.clone(), name: name.clone() },
        Admin::Rename { old, new } => Op::Rename { old: old.clone(), new: new.clone() },
        Admin::SetCountry { who, code } => Op::SetCountry { who: who.clone(), code: code.clone() },
        Admin::Countries => Op::Countries,
        Admin::GrantTitle { player, title } => Op::GrantTitle { player: player.clone(), title: title.clone() },
        Admin::Titles => Op::Titles,
        Admin::GiveBp { who, amount } => Op::GiveBp { who: who.clone(), amount: *amount },
        Admin::GiveCards { player, card, art, count } => Op::GiveCards { player: player.clone(), card: card.clone(), art: *art, count: *count },
        Admin::GiveRecipe { player, deck, cards } => Op::GiveRecipe { player: player.clone(), deck: deck.clone(), cards: *cards },
        Admin::Recipes => Op::Recipes,
        Admin::AddCode { code, bp, uses } => Op::AddCode { code: code.clone(), bp: *bp, uses: *uses },
        Admin::RemoveCode { code } => Op::RemoveCode { code: code.clone() },
        Admin::Codes => Op::Codes,
        Admin::Invite { uses, days } => Op::Invite { uses: (*uses > 0).then_some(*uses), days: *days },
        Admin::Invites => Op::Invites,
        Admin::ResetCode { user } => Op::ResetCode { user: user.clone() },
        Admin::SetRole { user, role } => Op::SetRole { user: user.clone(), role: role.clone() },
        // The password is made and hashed here: only its hash goes to the server.
        Admin::AddUser { user } => Op::AddUser { user: user.clone(), hash: String::new() },
        Admin::Settings | Admin::Share { .. } => Op::Settings,
        Admin::Set { name, value } => Op::Set { name: name.clone(), value: value.clone() },
        Admin::Reset { name } => Op::Reset { name: name.clone() },
        Admin::Audit { limit } => Op::Audit { limit: Some(*limit) },
        Admin::Stop => Op::Shutdown,
        Admin::Kick { who } => Op::Kick { who: who.clone() },
        Admin::Ban { who, reason, days } => Op::Ban { who: who.clone(), reason: reason.clone(), days: *days },
        Admin::Restrict { who, reason, days } => Op::Restrict { who: who.clone(), reason: reason.clone(), days: *days },
        Admin::Lift { who } => Op::Lift { who: who.clone() },
        Admin::Mute { who, reason, minutes } => Op::Mute { who: who.clone(), reason: reason.clone(), minutes: *minutes },
        Admin::Unmute { who } => Op::Unmute { who: who.clone() },
        Admin::SignOut { who } => Op::SignOut { who: who.clone() },
        Admin::Maintenance { minutes, message, off } => Op::Maintenance { minutes: *minutes, message: message.clone(), off: *off },
    }))
}

/// Runs an admin command: through the running server's admin API when one answers for this data folder, else on the
/// folder itself (holding its lock, as a server does), through the same operation. Both write the audit log.
fn admin(a: &Args, command: Admin, game_dir: PathBuf) -> anyhow::Result<()> {
    match &command {
        Admin::InitAuth { names } => {
            let tls = yo3_server::auth::initialize(&a.data_dir, names.clone())?;
            println!("Public certificate: {}\nPrivate server key: {}", tls.certificate.display(), tls.private_key.display());
            return Ok(());
        }
        // Restoring comes before opening the database: the one being replaced may be the damaged one.
        Admin::Restore { which } => {
            let _lock = yo3_server::store::lock(&a.data_dir)?;
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
    let mut op = operation(&command)?.expect("an API operation");
    let mut handed = None;
    if let Op::AddUser { user, hash } = &mut op {
        let password = yo3_server::logins::new_password();
        *hash = yo3_server::logins::hash_password_now(&password)?;
        handed = Some(format!("User ID: {user}\nPassword: {password} (they change it in the launcher)"));
    }
    if let Admin::Share { .. } = &command {
        // Before any request: what friends dial is the host's to say.
        anyhow::ensure!(a.advertise.is_some(), "pass --advertise ADDRESS: the address friends reach this server at (an IP or a name in its certificate)");
    }
    let mut result = call(a, &op, &game_dir)?;
    if let Admin::Share { invite: true } = &command {
        result["invite"] = call(a, &Op::Invite { uses: Some(1), days: Some(7) }, &game_dir)?;
    }
    show(a, &command, &result, handed)
}

/// One admin operation, on the running server or on its stopped data folder.
fn call(a: &Args, op: &Op, game_dir: &std::path::Path) -> anyhow::Result<Value> {
    match live::running(&a.data_dir) {
        Some(server) => api::call_as(&server, op, &a.by),
        None => {
            if matches!(op, Op::Shutdown | Op::Kick { .. }) { anyhow::bail!("no server is running on {}", a.data_dir.display()) }
            let _lock = if op.changes() { Some(yo3_server::store::lock(&a.data_dir)?) } else { None };
            yo3_server::store::initialize(&a.data_dir)?;
            let accounts = Directory::open(a.players.clone(), &a.data_dir.join("accounts.json"))?;
            let settings = offline_settings(&a.data_dir, game_dir, matches!(op, Op::Set { .. }))?;
            let offline = api::Api::new(Arc::new(Mutex::new(accounts)), Arc::new(settings), Some(a.data_dir.clone()), game_dir.to_path_buf(), "offline");
            offline.apply(op, &a.by)
        }
    }.map_err(anyhow::Error::msg)
}

/// Prints what an operation answered, as the host reads it.
fn show(a: &Args, command: &Admin, result: &Value, handed: Option<String>) -> anyhow::Result<()> {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    match command {
        Admin::Profile { output, .. } => {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)] {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options.open(output)?.write_all(profile(a, &text(&result["key"])).as_bytes())?;
        }
        Admin::Key { .. } | Admin::AddAccount { .. } => print_key(a, &text(&result["key"])),
        Admin::Share { .. } => {
            let name = result["settings"].as_array().into_iter().flatten().find(|r| r["name"] == "server-name").map(|r| text(&r["value"])).unwrap_or_default();
            let certificate = a.auth_cert.clone().unwrap_or_else(|| a.data_dir.join("server.crt"));
            let pem = std::fs::read(&certificate).map_err(|e| anyhow::anyhow!("{}: {e} (run init-auth first)", certificate.display()))?;
            let identity = yo3_tunnel::identity_from_pem(&pem)?;
            let host = a.advertise.as_deref().unwrap_or_default();
            let address = match host.parse::<IpAddr>() { Ok(IpAddr::V6(_)) => format!("[{host}]:{}", a.auth_port), _ => format!("{host}:{}", a.auth_port) };
            let invite = yo3_tunnel::invite::Invite::new(&address, &identity, &name)?;
            println!("Server: {}\nAddress: {}\nIdentity: {}\nInvite link: {}", invite.name, invite.address, invite.identity, invite.link());
            if let Some(code) = result["invite"]["code"].as_str() { println!("Invite code: {code} (one sign-up, for a week)") }
            println!("\nFriends paste the link under Multiplayer, Other servers (Desktop), or sign up with it in a terminal (Scripts):\n  \
                play --register \"LINK\" USER --profile client.json");
            if result["invite"].is_null() { println!("Each friend also needs an invite code if registration is invite (the default): add --invite, or run invite.") }
        }
        Admin::Accounts => for account in result.as_array().into_iter().flatten() {
            let names: Vec<_> = account["players"].as_array().into_iter().flatten().map(|p| format!("{} (#{})", text(&p["name"]), p["player_id"])).collect();
            let country = Some(text(&account["country"])).filter(|c| !c.is_empty()).unwrap_or_else(|| "auto".into());
            let found_by = match (account["login"].as_str(), account["has_key"] == true) {
                (Some(user), _) => format!("User ID {user}\t{}", text(&account["role"])), (None, true) => "key".into(), (None, false) => "-".into() };
            println!("user {}\t{found_by}\tcountry {country}\t{}", account["user_id"], names.join(", "));
        },
        Admin::Codes => {
            let codes = result.as_array().cloned().unwrap_or_default();
            if codes.is_empty() { println!("no codes (add one with add-code)") }
            for c in codes {
                let used = c["claims"].as_array().map_or(0, Vec::len);
                let left = c["uses"].as_u64().map_or("unlimited".to_string(), |n| format!("{} of {n} left", n.saturating_sub(used as u64)));
                println!("{}\t{} BP\tused by {used} account(s)\t{left}", text(&c["code"]), c["bp"]);
                for claim in c["claims"].as_array().into_iter().flatten() {
                    let players: Vec<String> = claim["players"].as_array().into_iter().flatten().map(text).collect();
                    let at = claim["at"].as_str().unwrap_or("an unrecorded time");
                    println!("  claimed by user {} ({}) at {at}", claim["user_id"], players.join(", "));
                }
            }
        }
        Admin::Audit { .. } => {
            let rows = result["rows"].as_array().cloned().unwrap_or_default();
            if rows.is_empty() { println!("no changes yet") }
            for r in rows {
                let value = |v: &Value| if v.is_null() { String::new() } else { v.to_string() };
                let change = if r["before"].is_null() && r["after"].is_null() { String::new() } else { format!("\t{} -> {}", value(&r["before"]), value(&r["after"])) };
                println!("{}\t{} ({})\t{}\t{}{change}", text(&r["at"]), text(&r["actor"]), text(&r["origin"]), text(&r["op"]), text(&r["target"]));
            }
        }
        _ => {
            if let Some(handed) = handed { println!("{handed}") }
            match result["message"].as_str() { Some(m) => println!("{m}"), None => println!("{}", serde_json::to_string_pretty(result)?) }
        }
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
        .with_writer(move || yo3_server::recent::Tee::new(yo3_tunnel::diagnostics::ConsoleWriter::new(writer.clone()).colored(color)))
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
        packet_logging: !a.no_packet_log, packet_traces: !a.no_packet_trace, diagnostic_id: log.as_ref().map(|l| l.id()),
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
        identity_check: yo3_server::IDENTITY_CHECK,
    };
    let mut running = match start(cfg).await { Ok(running) => running, Err(error) => { tracing::error!("Server startup failed: {error:#}"); return Err(error) } };
    if a.owned {
        // Standard input stays open while the owning launcher lives; end of input means it has gone.
        let owner_gone = async {
            let _ = tokio::task::spawn_blocking(|| { let _ = std::io::copy(&mut std::io::stdin().lock(), &mut std::io::sink()); }).await;
        };
        running.exit_when_idle_without_owner(owner_gone, std::time::Duration::from_secs(a.owned_idle));
    }
    // SIGTERM (a service manager, `docker stop`) and Ctrl+C stop the server gracefully, as `yo3-server stop` does.
    let stop = running.stopper();
    tokio::spawn(async move {
        stop_signal().await;
        tracing::info!("stop signal received");
        stop.send_replace(true);
    });
    running.wait().await;
    tracing::info!("stopped");
    Ok(())
}

/// Ctrl+C, or on Unix SIGTERM.
async fn stop_signal() {
    #[cfg(unix)]
    {
        let Ok(mut term) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) else {
            let _ = tokio::signal::ctrl_c().await;
            return;
        };
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
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
