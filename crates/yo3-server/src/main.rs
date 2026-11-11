use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;
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

/// Runs an admin command against persisted accounts, or for the BP commands, the running server.
fn admin(a: &Args, command: Admin, game_dir: PathBuf) -> anyhow::Result<()> {
    yo3_server::store::initialize(&a.data_dir)?;
    if let Admin::InitAuth { names } = command {
        let tls = yo3_server::auth::initialize(&a.data_dir, names)?;
        println!("Public certificate: {}\nPrivate server key: {}", tls.certificate.display(), tls.private_key.display());
        return Ok(());
    }
    let op = match &command {
        Admin::GiveBp { who, amount } => Some(Op::GiveBp { who: who.clone(), amount: *amount }),
        Admin::AddCode { code, bp, uses } => Some(Op::AddCode { code: code.clone(), bp: *bp, uses: *uses }),
        Admin::RemoveCode { code } => Some(Op::RemoveCode { code: code.clone() }),
        Admin::Codes => Some(Op::Codes),
        _ => None,
    };
    let server = live::running(&a.data_dir);
    if let Some(op) = op {
        let done = match &server {
            Some(server) => live::send(server, &op),
            None => live::apply(&mut Directory::open(a.players.clone(), &a.data_dir.join("accounts.json"))?, &op),
        };
        println!("{}", done.map_err(anyhow::Error::msg)?);
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
        Admin::GiveBp { .. } | Admin::AddCode { .. } | Admin::RemoveCode { .. } | Admin::Codes => unreachable!("handled above"),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut a = Args::parse();
    let log = if a.admin.is_none() { Some(yo3_tunnel::diagnostics::Session::start(&a.diagnostics_dir.clone().unwrap_or_else(|| a.log_dir.join("sessions")), "server", a.diagnostic_parent.clone())?) } else { None };
    let writer = log.clone();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_target(false)
        .with_ansi(false)
        .with_writer(move || yo3_tunnel::diagnostics::ConsoleWriter::new(writer.clone()))
        .init();
    if let Some(command) = a.admin.take() {
        let game_dir = a.game_dir.clone().unwrap_or_else(yo3_data::default_game_dir);
        return admin(&a, command, game_dir);
    }
    let cfg = Config {
        tls: yo3_server::auth::TlsConfig { port: a.auth_port,
            certificate: a.auth_cert.unwrap_or_else(|| a.data_dir.join("server.crt")),
            private_key: a.auth_key.unwrap_or_else(|| a.data_dir.join("server.key")) },
        bind: a.bind,
        advertise: a.advertise,
        http_port: a.http_port,
        game_dir: a.game_dir.unwrap_or_else(yo3_data::default_game_dir),
        log_dir: a.log_dir,
        packet_logging: !a.no_packet_log, diagnostic_id: log.as_ref().map(|l| l.id()),
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
        duel_check: a.duel_check,
    };
    let running = match start(cfg).await { Ok(running) => running, Err(error) => { tracing::error!("Server startup failed: {error:#}"); return Err(error) } };
    running.wait().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_ipv6_profile_has_a_bracketed_address_and_an_unbracketed_certificate_name() {
        let args = Args::try_parse_from(["yo3-server", "--advertise", "::1"]).unwrap();
        let p: serde_json::Value = serde_json::from_str(&profile(&args, "private-test-key")).unwrap();
        assert_eq!(p["server"], "[::1]:7443");
        assert_eq!(p["server_name"], "::1");
    }
}
