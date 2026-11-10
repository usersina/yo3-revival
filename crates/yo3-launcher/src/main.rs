use std::path::PathBuf;
#[cfg(not(windows))]
use std::path::Path;
use std::process::Stdio;
use anyhow::{Context, ensure};
use clap::Parser;
use yo3_launcher::{Ports, Profile, Proxy, hex};

#[derive(Parser)]
#[command(about = "Authenticate, start private TLS proxies, and inject the revival connection mod into the game")]
struct Args {
    #[arg(long)] profile: PathBuf,
    #[arg(long, required_unless_present = "proxy_only")] game_dir: Option<PathBuf>,
    #[arg(long)] injector: Option<PathBuf>,
    #[arg(long)] dll: Option<PathBuf>,
    #[arg(long)] trace_dll: Option<PathBuf>,
    #[arg(long)] wine_prefix: Option<PathBuf>,
    /// Run the authenticated proxy for protocol tests, without starting the game.
    #[arg(long)] proxy_only: bool,
    /// Private JSON endpoint descriptor for protocol tests (contains the local cookie).
    #[arg(long)] ready_file: Option<PathBuf>,
    #[arg(long, default_value_t = 0)] gate_port: u16,
    #[arg(long, default_value_t = 0)] front_port: u16,
    #[arg(long, default_value_t = 0)] duel_port: u16,
    #[arg(long, default_value_t = 0)] http_port: u16,
}

#[cfg(not(windows))]
fn windows_path(path: &Path, prefix: Option<&Path>) -> anyhow::Result<String> {
    let mut cmd = std::process::Command::new("winepath");
    cmd.args(["-w"]).arg(path).stderr(Stdio::null());
    if let Some(prefix) = prefix { cmd.env("WINEPREFIX", prefix); }
    let out = cmd.output()?;
    ensure!(out.status.success(), "winepath failed");
    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let (endpoint, key) = Profile::load(&args.profile).context("read the private launcher profile and host certificate")?;
    let session = endpoint.login(&key).await.context("authentication failed; the game was not started")?;
    let proxy = Proxy::start(endpoint.clone(), session, Ports { gate: args.gate_port, front: args.front_port, duel: args.duel_port, http: args.http_port }).await?;
    if let Some(file) = &args.ready_file {
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new(); options.create_new(true).write(true);
        #[cfg(unix)] {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let data = serde_json::json!({ "ports": proxy.ports, "cookie": hex(&proxy.cookie) });
        options.open(file)?.write_all(data.to_string().as_bytes())?;
    }
    eprintln!("Authenticated. Local gate {}, front {}, duel {}, HTTP {}.", proxy.ports.gate, proxy.ports.front, proxy.ports.duel, proxy.ports.http);
    if args.proxy_only { tokio::signal::ctrl_c().await?; let _ = endpoint.logout(&session).await; return Ok(()) }
    let game = args.game_dir.unwrap().join("yo3.exe").canonicalize().context("find yo3.exe in --game-dir")?;
    let bin = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let injector = args.injector.unwrap_or_else(|| bin.join("yo3-auth-launch.exe")).canonicalize().context("find the 32-bit injector")?;
    let dll = args.dll.unwrap_or_else(|| bin.join("yo3-auth.dll")).canonicalize().context("find the connection DLL")?;
    #[cfg(windows)]
    let mut cmd = { let mut cmd = tokio::process::Command::new(injector); cmd.arg(game).arg(dll); cmd };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut cmd = tokio::process::Command::new("wine");
        cmd.arg(injector).arg(windows_path(&game, args.wine_prefix.as_deref())?).arg(windows_path(&dll, args.wine_prefix.as_deref())?);
        if let Some(prefix) = &args.wine_prefix { cmd.env("WINEPREFIX", prefix); }
        cmd
    };
    if let Some(trace) = args.trace_dll {
        #[cfg(windows)] cmd.arg(trace);
        #[cfg(not(windows))] cmd.arg(windows_path(&trace.canonicalize()?, args.wine_prefix.as_deref())?);
    }
    cmd.envs(proxy.environment()).stdin(Stdio::null()).kill_on_drop(true);
    let mut child = cmd.spawn().context("start the injector")?;
    let status = tokio::select! {
        status = child.wait() => Some(status?),
        _ = tokio::signal::ctrl_c() => { child.kill().await?; None }
    };
    let _ = endpoint.logout(&session).await;
    if let Some(status) = status { ensure!(status.success(), "game or injector exited with an error"); }
    Ok(())
}
