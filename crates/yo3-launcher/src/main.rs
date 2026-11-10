use std::path::PathBuf;
#[cfg(not(windows))]
use std::path::Path;
use std::process::Stdio;
use anyhow::{Context, ensure};
use clap::Parser;
use yo3_launcher::{Ports, Profile, Proxy, hex};

#[derive(Parser)]
#[command(version = yo3_tunnel::BUILD_VERSION, about = "Authenticate, start private TLS proxies, and inject the revival connection mod into the game")]
struct Args {
    #[arg(long)] profile: PathBuf,
    #[arg(long, required_unless_present = "proxy_only")] game_dir: Option<PathBuf>,
    #[arg(long)] injector: Option<PathBuf>,
    #[arg(long)] dll: Option<PathBuf>,
    #[arg(long)] trace_dll: Option<PathBuf>,
    #[arg(long)] wine_prefix: Option<PathBuf>,
    /// Desktop lifecycle pipe. Closing stdin or sending `stop` closes the game and revokes its session.
    #[arg(long)] control_stdin: bool,
    #[arg(long)] diagnostics_dir: Option<PathBuf>,
    #[arg(long)] diagnostic_parent: Option<String>,
    /// Include verbose client trace output. Review reports for player/chat information before sharing.
    #[arg(long)] detailed_logs: bool,
    /// A game-local bootstrap attaches to this existing game after proving mod readiness through private files.
    #[arg(long, requires = "proxy_only")] attach_game: Option<u32>,
    #[arg(long, requires = "attach_game")] mod_ready_file: Option<PathBuf>,
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
    let report = args.attach_game.and(args.ready_file.clone());
    let directory = args.diagnostics_dir.clone().unwrap_or_else(|| args.profile.parent().unwrap_or(std::path::Path::new(".")).join("logs/sessions"));
    let log = yo3_tunnel::diagnostics::Session::start(&directory, if args.attach_game.is_some() { "direct" } else { "game" }, args.diagnostic_parent.clone())?;
    yo3_tunnel::diagnostics::set_current(log.clone());
    let result = run(args, log.clone()).await;
    if let Err(error) = &result { log.line(&format!("Startup/connection failed: {error:#}")); }
    else { log.line("Game/helper session ended"); }
    if let (Some(report), Err(error)) = (report, &result) { yo3_launcher::attach::report_error(&report, &format!("{error:#}")); }
    result
}

async fn run(args: Args, log: yo3_tunnel::diagnostics::Session) -> anyhow::Result<()> {
    let bin = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let injector = args.injector.clone().unwrap_or_else(|| bin.join("yo3-auth-launch.exe"));
    let dll = args.dll.clone().unwrap_or_else(|| bin.join("yo3-auth.dll"));
    let game = args.game_dir.as_ref().map(|dir| dir.join("yo3.exe"));
    if !args.proxy_only || args.attach_game.is_some() { yo3_launcher::components::preflight(game.as_ref().context("Direct game attachment requires --game-dir")?, &injector, &dll)?; }
    if let Some(trace) = &args.trace_dll { ensure!(trace.is_file(), "Trace DLL not found: {}", trace.display()); }
    let (endpoint, key) = Profile::load(&args.profile).context("read the private launcher profile and host certificate")?;
    log.secret(&key); log.line("Checking server identity and compatibility");
    endpoint.info().await?.ensure_compatible(yo3_tunnel::PROTOCOL_VERSION, yo3_tunnel::MOD_ABI)?;
    let proxy = Proxy::prepare(endpoint.clone(), Ports { gate: args.gate_port, front: args.front_port, duel: args.duel_port, http: args.http_port }).await?;
    log.secret(&hex(&proxy.cookie));
    if let Some(pid) = args.attach_game {
        let request = yo3_launcher::attach::Attachment { pid, game: game.unwrap(),
            ready: args.ready_file.context("Direct game attachment requires a private ready file")?,
            ack: args.mod_ready_file.context("Direct game attachment requires a private acknowledgement file")? };
        log.line("Waiting for the direct loader to acknowledge mod readiness");
        return yo3_launcher::attach::serve(request, endpoint, &key, proxy).await;
    }
    if args.proxy_only {
        let session = endpoint.login(&key).await?; proxy.activate(session);
        let result = async { write_ready(&args, &proxy)?; tokio::signal::ctrl_c().await?; Ok(()) }.await;
        let _ = endpoint.logout(&session).await; return result;
    }
    let game = game.unwrap().canonicalize()?;
    let injector = injector.canonicalize()?; let dll = dll.canonicalize()?;
    #[cfg(windows)]
    let mut cmd = { let mut cmd = tokio::process::Command::new(injector); cmd.arg("--control-stdin").arg(game).arg(dll); cmd };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut cmd = tokio::process::Command::new("wine");
        cmd.arg(injector).arg("--control-stdin").arg(windows_path(&game, args.wine_prefix.as_deref())?).arg(windows_path(&dll, args.wine_prefix.as_deref())?);
        if let Some(prefix) = &args.wine_prefix { cmd.env("WINEPREFIX", prefix); }
        cmd
    };
    if let Some(trace) = &args.trace_dll {
        #[cfg(windows)] cmd.arg(trace);
        #[cfg(not(windows))] cmd.arg(windows_path(&trace.canonicalize()?, args.wine_prefix.as_deref())?);
    }
    cmd.envs(proxy.environment()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let mut child = cmd.spawn().context("start the injector")?;
    let errors = child.stderr.take().unwrap(); let error_log = log.clone();
    let error_output = tokio::spawn(async move { capture(errors, error_log, true).await; });
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stdout = tokio::io::BufReader::new(child.stdout.take().unwrap());
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut ready = Vec::new();
        for _ in 0..128 { let byte = stdout.read_u8().await.context("Game stayed suspended: the mod did not report readiness. Repair the connection components.")?; if byte == b'\n' { break } ready.push(byte); }
        if ready.last() == Some(&b'\r') { ready.pop(); }
        ensure!(ready == b"YO3_MOD_READY", "Invalid injector readiness response; game was not resumed");
        Ok::<_, anyhow::Error>(())
    }).await.context("Connection mod initialization timed out; game was not resumed")??;
    log.line("Connection DLL loaded and interface ready; authenticating");
    let session = endpoint.login(&key).await.context("Authentication failed; game was not resumed")?; log.secret(&hex(&session));
    proxy.activate(session);
    let game_log = log.clone(); let detailed = args.detailed_logs;
    let output = tokio::spawn(async move { capture(stdout, game_log, detailed).await; });
    let result = async {
        child.stdin.as_mut().unwrap().write_all(b"resume\n").await?;
        child.stdin.take();
        write_ready(&args, &proxy)?;
        eprintln!("Authenticated with mod interface {} ready. Local gate {}, front {}, duel {}, HTTP {}.", yo3_tunnel::MOD_ABI, proxy.ports.gate, proxy.ports.front, proxy.ports.duel, proxy.ports.http);
        log.line("Authenticated; game resumed");
        let desktop_stop = async {
            if args.control_stdin { yo3_launcher::wait_for_desktop_stop(std::io::BufReader::new(std::io::stdin())).await; }
            else { std::future::pending::<()>().await; }
        };
        let status = tokio::select! {
            status = child.wait() => Some(status?),
            _ = tokio::signal::ctrl_c() => { child.kill().await?; None }
            _ = desktop_stop => { child.kill().await?; None }
        };
        if let Some(status) = status { ensure!(status.success(), "game or injector exited with an error"); }
        Ok(())
    }.await;
    let _ = endpoint.logout(&session).await;
    if child.try_wait()?.is_none() { let _ = child.kill().await; }
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), output).await;
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), error_output).await;
    result
}

async fn capture(mut source: impl tokio::io::AsyncRead + Unpin, log: yo3_tunnel::diagnostics::Session, detailed: bool) {
    use tokio::io::AsyncReadExt;
    let mut buffer = [0; 4096]; let mut writer = log.writer();
    // Normal stdout is drained: the original client prints raw packets containing chat/player data.
    while let Ok(n) = source.read(&mut buffer).await {
        if n == 0 { break }
        if detailed { let _ = std::io::Write::write_all(&mut writer, &buffer[..n]); }
    }
}

fn write_ready(args: &Args, proxy: &Proxy) -> anyhow::Result<()> {
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
    Ok(())
}
