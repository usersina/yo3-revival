use std::path::PathBuf;
use std::process::Stdio;
use anyhow::{Context, ensure};
use clap::Parser;
use yo3_launcher::{Ports, Profile, Proxy, hex};
#[cfg(not(windows))]
use yo3_launcher::runtime::windows_path;

#[derive(Parser)]
#[command(version = yo3_tunnel::BUILD_VERSION, about = "Authenticate, start private TLS proxies, and inject the revival connection mod into the game")]
struct Args {
    #[arg(long, required_unless_present_any = ["vault_set", "vault_delete"])] profile: Option<PathBuf>,
    /// Sign in to SERVER (its invite link, or its address with --identity) as USER, asking for the password, and keep
    /// this PC signed in: the device token goes in the system's credential store and --profile becomes a profile
    /// without a secret, which `play` uses. A host's `yo3-server share` prints the link.
    #[arg(long, num_args = 2, value_names = ["SERVER", "USER"], conflicts_with_all = ["register", "recover", "vault_set", "vault_delete"])] sign_in: Option<Vec<String>>,
    /// Make an account USER on SERVER (asking for the host's invite code, a password and the server's rules), then sign in
    /// as --sign-in does.
    #[arg(long, num_args = 2, value_names = ["SERVER", "USER"], conflicts_with_all = ["recover", "vault_set", "vault_delete"])] register: Option<Vec<String>>,
    /// Choose a password with a setup, recovery or moderator reset code, then sign in. All secrets are prompted.
    #[arg(long, num_args = 2, value_names = ["SERVER", "USER"], conflicts_with_all = ["sign_in", "register", "vault_set", "vault_delete"])] recover: Option<Vec<String>>,
    /// The server's identity (SHA-256 in hex) when --sign-in, --register or --recover names it by address.
    #[arg(long, value_name = "HEX")] identity: Option<String>,
    /// Keep a device token (read from standard input) in this system's credential vault under NAME: the Linux launcher
    /// copies its saved sign-in into Wine's store for direct start (#33).
    #[arg(long, value_name = "NAME", conflicts_with = "vault_delete")] vault_set: Option<String>,
    /// Remove the device token kept under NAME.
    #[arg(long, value_name = "NAME")] vault_delete: Option<String>,
    #[arg(long, required_unless_present_any = ["proxy_only", "vault_set", "vault_delete", "sign_in", "register", "recover"])] game_dir: Option<PathBuf>,
    #[arg(long)] injector: Option<PathBuf>,
    #[arg(long)] dll: Option<PathBuf>,
    #[arg(long)] trace_dll: Option<PathBuf>,
    /// Draw the game larger, sharp and in 4:3: WIDTHxHEIGHT for a window, auto for the largest the screen holds, or off
    #[arg(long)] resolution: Option<yo3_launcher::resolution::Resolution>,
    /// The anime artwork folder (#60): the connection DLL shows `card/<Konami id>.png` for a card the host's card profile
    /// shows as its anime version; thumbnails are made from the faces that lack one. Created if missing.
    #[arg(long, value_name = "DIR")] anime_art: Option<PathBuf>,
    /// The game's original `Yu-Gi-Oh! ONLINE 3` Documents folder: before the game is released, the player's local decks are
    /// placed as the server's card profile says (decks.rs, issue #60).
    #[arg(long, value_name = "DIR")] client_data: Option<PathBuf>,
    #[arg(long)] wine_prefix: Option<PathBuf>,
    /// Desktop lifecycle pipe. Closing stdin or sending `stop` closes the game and revokes its session.
    #[arg(long)] control_stdin: bool,
    #[arg(long)] diagnostics_dir: Option<PathBuf>,
    /// Where the connection DLL writes its report of what else runs inside the game (#68), and where this writes what it
    /// decided: without it, a file beside the session log, removed at the end.
    #[arg(long)] survey_file: Option<PathBuf>,
    /// Start the game although another program's file or provider was found (the player's Continue anyway). A redirected
    /// network function still stops it.
    #[arg(long)] accept_foreign: bool,
    /// Send no problem report: by default, after a game whose connection broke or that exited with an error, its session
    /// log goes to the server it played on, for the host (the same as a `problem-reports.off` file beside the profile).
    #[arg(long)] no_problem_reports: bool,
    /// The server asks for a clean game (#69): other programs' files or network software found in the game stop the
    /// start, and --accept-foreign does not go past them.
    #[arg(long)] clean_game: bool,
    /// An extra game from the Desktop's Play as… (#64): its game data folder is the server's and this key's own
    /// (the account), so the games' options, decks and duel logs stay apart.
    #[arg(long)] data_key: Option<String>,
    #[arg(long)] diagnostic_parent: Option<String>,
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

#[cfg(windows)]
fn windows_injector(injector: &std::path::Path, game: &std::path::Path, dll: &std::path::Path, compatibility: Option<std::ffi::OsString>) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(injector);
    // The original game's administrator manifest otherwise elevates it beyond the
    // injector. It only needs the current user's rights in the selected game copy.
    let mut layers = compatibility.unwrap_or_default();
    if !layers.is_empty() { layers.push(" "); }
    layers.push("RunAsInvoker");
    cmd.env("__COMPAT_LAYER", layers);
    cmd.arg("--control-stdin").arg(game).arg(dll); cmd
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.vault_set.is_some() || args.vault_delete.is_some() { return vault(&args) }
    if args.sign_in.is_some() || args.register.is_some() || args.recover.is_some() { return sign_in(&args).await }
    let profile = args.profile.clone().context("--profile is required")?;
    let report = args.attach_game.and(args.ready_file.clone());
    let directory = args.diagnostics_dir.clone().unwrap_or_else(|| profile.parent().unwrap_or(std::path::Path::new(".")).join("logs/sessions"));
    let log = yo3_tunnel::diagnostics::Session::start(&directory, if args.attach_game.is_some() { "direct" } else { "game" }, args.diagnostic_parent.clone())?;
    yo3_tunnel::diagnostics::set_current(log.clone());
    let result = run(args, log.clone()).await;
    if let Err(error) = &result { log.error(&format!("Startup/connection failed: {error:#}")); }
    else { log.line("Game/helper session ended"); }
    if let (Some(report), Err(error)) = (report, &result) { yo3_launcher::attach::report_error(&report, &format!("{error:#}")); }
    result
}

async fn run(args: Args, log: yo3_tunnel::diagnostics::Session) -> anyhow::Result<()> {
    let desktop_stop=async {
        if args.control_stdin {yo3_launcher::wait_for_desktop_stop(std::io::BufReader::new(std::io::stdin())).await}
        else {std::future::pending::<yo3_launcher::DesktopStop>().await}
    };
    tokio::pin!(desktop_stop);
    let interrupt=tokio::signal::ctrl_c();tokio::pin!(interrupt);
    let startup_deadline=tokio::time::sleep(std::time::Duration::from_secs(90));tokio::pin!(startup_deadline);
    // Keep one control-pipe reader throughout startup and play. Dropping a pending
    // initializer also drops its kill-on-drop injector/Wine child and private proxy.
    macro_rules! initializing {
        ($future:expr) => {tokio::select! {
            biased;
            _=&mut desktop_stop=>{log.line("Game startup cancelled");return Ok(())},
            result=&mut interrupt=>{result?;log.line("Game startup interrupted");return Ok(())},
            _=&mut startup_deadline=>anyhow::bail!("Game startup did not finish within 90 seconds. Close any stuck Wine processes for this game, validate the game and try again. Export logs if this repeats."),
            result=$future=>result,
        }};
    }
    let bin = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let injector = args.injector.clone().unwrap_or_else(|| bin.join("yo3-auth-launch.exe"));
    let dll = args.dll.clone().unwrap_or_else(|| bin.join("yo3-auth.dll"));
    let game = args.game_dir.as_ref().map(|dir| dir.join("yo3.exe"));
    if !args.proxy_only || args.attach_game.is_some() {
        let game = game.as_ref().context("Direct game attachment requires --game-dir")?;
        let info = yo3_launcher::components::preflight(game, &injector, &dll)?;
        // Group the fingerprint so conservative credential redaction preserves this public diagnostic.
        let digest = yo3_launcher::components::inspect_game(game)?;
        let fingerprint = digest.as_bytes().chunks(16).map(|p| std::str::from_utf8(p).unwrap()).collect::<Vec<_>>().join(":");
        log.line(&format!("Game v1.153 hook layout supported; executable sha256={fingerprint}; mod release={} commit={} ABI={}", info.release, info.commit, info.abi));
        if args.attach_game.is_none() {
            log.line("Checking game runtime prerequisites without starting the game");
            initializing!(yo3_launcher::runtime::check(game, &injector, args.wine_prefix.as_deref()))?;
            log.line("Game runtime prerequisites available");
        }
    }
    if let Some(trace) = &args.trace_dll { ensure!(trace.is_file(), "Trace DLL not found: {}", trace.display()); }
    let profile = args.profile.clone().context("--profile is required")?;
    let loaded = Profile::read(&profile).context("read the private launcher profile and host certificate")?;
    let (endpoint, key, vault) = (loaded.endpoint.clone(), loaded.credential.clone(), loaded.vault.clone());
    log.secret(&key.secret());
    // The server's own word on a clean game (#69), besides the flag the Desktop gives: Scripts plays by this too.
    let mut clean_game = args.clean_game;
    // Direct start checks the server after the game is released: a refusal is the game's own login dialog (#33).
    if args.attach_game.is_none() {
        log.line("Checking server identity and compatibility");
        let info=initializing!(endpoint.info())?;
        info.ensure_compatible(yo3_tunnel::PROTOCOL_VERSION, yo3_tunnel::MOD_ABI)?;
        info.ensure_launcher(yo3_tunnel::BUILD_VERSION)?;
        clean_game |= info.clean_game;
    }
    let mut proxy = initializing!(Proxy::prepare(endpoint.clone(), Ports { gate: args.gate_port, front: args.front_port, duel: args.duel_port, http: args.http_port }))?;
    log.secret(&hex(&proxy.cookie));
    if let Some(key) = &args.data_key { proxy.server_id = yo3_launcher::game_data_id(&proxy.server_id, key); log.line("This game keeps its own game data folder") }
    // The game's patch check gets the game's own index on every Play, never a server's (security review, #39).
    if let Some(dir) = &args.game_dir { proxy.local_index_from(dir) }
    if let Some(pid) = args.attach_game {
        let request = yo3_launcher::attach::Attachment { pid, game: game.unwrap(),
            ready: args.ready_file.context("Direct game attachment requires a private ready file")?,
            ack: args.mod_ready_file.context("Direct game attachment requires a private acknowledgement file")? };
        log.line("Waiting for the direct loader to acknowledge mod readiness");
        return yo3_launcher::attach::serve(request, loaded, &profile, proxy, log.clone(), !args.no_problem_reports && !yo3_launcher::problems::off(&profile)).await;
    }
    if args.proxy_only {
        let session = key.sign_in(&endpoint).await?; proxy.activate(session);
        let result = async { write_ready(&args, &proxy)?; (&mut interrupt).await?; Ok(()) }.await;
        if key.owns_session() { let _ = endpoint.logout(&session).await; } return result;
    }
    let game = yo3_launcher::runtime::native_path(&game.unwrap())?;
    let injector = yo3_launcher::runtime::native_path(&injector)?; let dll = yo3_launcher::runtime::native_path(&dll)?;
    #[cfg(windows)]
    let mut cmd = windows_injector(&injector, &game, &dll, std::env::var_os("__COMPAT_LAYER"));
    #[cfg(not(windows))]
    let mut cmd = {
        log.line("Preparing Wine paths");
        let mut cmd = tokio::process::Command::new(yo3_launcher::runtime::wine_tool("wine"));
        cmd.arg(injector).arg("--control-stdin").arg(initializing!(windows_path(&game, args.wine_prefix.as_deref()))?).arg(initializing!(windows_path(&dll, args.wine_prefix.as_deref()))?);
        if let Some(prefix) = &args.wine_prefix { cmd.env("WINEPREFIX", prefix); }
        cmd
    };
    if let Some(resolution) = args.resolution { cmd.env(yo3_launcher::resolution::ENV, resolution.to_string()); }
    // The anime artwork folder (#60): created, its thumbnails made, and told to the DLL. A folder that can't be made leaves the
    // game with its own artwork.
    if let Some(dir) = &args.anime_art {
        match yo3_launcher::art::prepare(dir) {
            Ok(done) => {
                log.line(&format!("Anime artwork: {} card face(s) in {}, {} thumbnail(s) made", done.faces.len(), dir.display(), done.made.len()));
                for (file, why) in &done.skipped { log.line(&format!("Anime artwork skipped: {} ({why})", file.display())); }
                cmd.env(yo3_launcher::art::ENV, dir);
            }
            Err(e) => log.warn(&format!("Anime artwork folder {} is not usable: {e}", dir.display())),
        }
    }
    // What else runs inside the game (#68): the DLL writes its report here before the game is released.
    let survey_given = args.survey_file.is_some();
    let survey_file = args.survey_file.clone().unwrap_or_else(|| args.diagnostics_dir.clone()
        .unwrap_or_else(|| profile.parent().unwrap_or(std::path::Path::new(".")).join("logs/sessions")).join(format!("{}.survey.json", log.id())));
    let _ = std::fs::remove_file(&survey_file);
    let _cleanup = (!survey_given).then(|| SurveyCleanup(survey_file.clone()));
    // The added cards (#75): the DLL reads which folder of extended card files to serve from this file, filled once signed in.
    let added_pointer = yo3_launcher::added::pointer_for(&yo3_launcher::added::base_for(profile.parent().unwrap_or(std::path::Path::new("."))), &log.id());
    let _added_cleanup = SurveyCleanup(added_pointer.clone());
    if yo3_launcher::added::point(&added_pointer, None).is_ok() {
        let named = yo3_launcher::runtime::native_path(added_pointer.parent().unwrap_or(std::path::Path::new(".")))?.join(added_pointer.file_name().context("the pointer names a file")?);
        #[cfg(windows)] cmd.env(yo3_launcher::added::ENV, &named);
        #[cfg(not(windows))] cmd.env(yo3_launcher::added::ENV, initializing!(windows_path(&named, args.wine_prefix.as_deref()))?);
    }
    {
        let parent = survey_file.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
        let named = yo3_launcher::runtime::native_path(parent)?.join(survey_file.file_name().context("--survey-file names a file")?);
        #[cfg(windows)] cmd.env("YO3_SURVEY_FILE", &named);
        #[cfg(not(windows))] cmd.env("YO3_SURVEY_FILE", initializing!(windows_path(&named, args.wine_prefix.as_deref()))?);
    }
    if let Some(trace) = &args.trace_dll {
        #[cfg(windows)] cmd.arg(yo3_launcher::runtime::native_path(trace)?);
        #[cfg(not(windows))] {let path=trace.canonicalize()?;cmd.arg(initializing!(windows_path(&path, args.wine_prefix.as_deref()))?);}
    }
    #[cfg(windows)] cmd.creation_flags(0x08000000);
    cmd.envs(proxy.environment()).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let mut child = cmd.spawn().context("start the injector")?;
    let errors = child.stderr.take().unwrap(); let error_log = log.clone();
    let mut error_output = tokio::spawn(async move { capture(errors, error_log, false).await });
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stdout = tokio::io::BufReader::new(child.stdout.take().unwrap());
    log.line("Starting the game suspended; waiting for the connection DLL");
    let readiness = initializing!(tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut ready = Vec::new();
        for _ in 0..128 { let byte = stdout.read_u8().await.context("Game stayed suspended: the mod did not report readiness. Repair the connection components.")?; if byte == b'\n' { break } ready.push(byte); }
        if ready.last() == Some(&b'\r') { ready.pop(); }
        ensure!(ready == b"YO3_MOD_READY", "Invalid injector readiness response; game was not resumed");
        Ok::<_, anyhow::Error>(())
    })).context("Connection mod initialization timed out; game was not resumed. Validate the game and check the required DLL, then try Start again").and_then(|result| result);
    if let Err(error) = readiness {
        let _ = child.kill().await;
        let detail = tokio::time::timeout(std::time::Duration::from_secs(1), &mut error_output).await.ok().and_then(Result::ok).unwrap_or_default();
        let clean = yo3_tunnel::diagnostics::redact(&detail, &[key.secret(), hex(&proxy.cookie)], None);
        let summary = clean.lines().filter(|line| line.starts_with("stage="))
            .find_map(|line| line.split_once(": ").map(|(_, message)| message))
            .unwrap_or("The game could not start safely. Validate the game and export logs if this repeats.");
        anyhow::bail!("{summary}\n{error:#}\n{}", if clean.trim().is_empty() { "The injector exited without a native diagnostic. Restore the matching application bundle, check security-software detection history and export logs for help." } else { clean.trim() });
    }
    log.line("Connection DLL loaded and interface ready; authenticating");
    // What else runs inside the game (#68): the player hears about another program's file or provider before the game is
    // released, and a redirected network function stops it. The DLL's own lines are in this log.
    if let Some(line) = yo3_launcher::survey::tools_line(&yo3_launcher::survey::running_tools()) { log.line(&format!("Survey: {line}")); }
    match yo3_launcher::survey::Survey::read(&survey_file) {
        Some(report) => {
            let verdict = report.assessment();
            let decision = verdict.decision_on(args.accept_foreign, clean_game);
            for finding in verdict.all() { log.line(&format!("Survey: {}", yo3_launcher::survey::describe(finding))); }
            log.line(&format!("Survey: decision {decision:?}"));
            let _ = yo3_launcher::survey::record_decision_on(&survey_file, decision, clean_game);
            if decision != yo3_launcher::survey::Decision::Went {
                let _ = child.kill().await;
                let what = verdict.all().map(yo3_launcher::survey::describe).collect::<Vec<_>>().join("; ");
                if decision == yo3_launcher::survey::Decision::Blocked && clean_game && verdict.blocked.is_empty() {
                    anyhow::bail!("The game was not started: this server asks for a clean game, and {what}. Remove it, then try again.");
                }
                if decision == yo3_launcher::survey::Decision::Blocked {
                    anyhow::bail!("The game was not started: {what}. It would break the game's connection. Close that program or remove its file, then try again.");
                }
                anyhow::bail!("The game was not started: {what}. Start it again with --accept-foreign to go on anyway.");
            }
        }
        None => log.line("Survey: the connection DLL wrote no report"),
    }
    let mut session = match &key {
        // A sign-in profile (--sign-in): without a saved sign-in, or with one the server refuses, the game still starts
        // and its own login dialog says why; with none kept, its login form asks (#34).
        yo3_launcher::Credential::Device(_) => {
            let read = |account: &str| yo3_launcher::vault::Vault::get(&yo3_launcher::vault::SystemVault, account).ok().flatten();
            match initializing!(async { Ok::<_, anyhow::Error>(yo3_launcher::direct::sign_in(&endpoint, &key, &read).await) })? {
                Ok(session) => Some(session),
                Err(failure) => {
                    log.warn(&format!("Sign-in refused ({:?}): {}", failure.refusal, failure.reason));
                    eprintln!("{}", match failure.refusal {
                        yo3_launcher::direct::Refusal::SignedOut => "No saved sign-in for this server: sign in with the game's own login form.".to_string(),
                        _ => format!("The saved sign-in was refused: {}", failure.reason),
                    });
                    if let Some(notice) = failure.refusal.notice() { eprintln!("{notice}") }
                    // The game's patch check is answered from its own index while there is no sign-in (Proxy::local_index).
                    if let Some(bytes) = args.game_dir.as_ref().and_then(|dir| std::fs::read(dir.join("yo3.idx")).ok()).filter(|b| b.len() <= 1 << 20) { proxy.local_index(bytes) }
                    proxy.refuse(failure.refusal);
                    None
                }
            }
        }
        _ => Some(initializing!(key.sign_in(&endpoint)).context("Authentication failed; game was not resumed. Check the profile and that the server is running, or export logs for help")?),
    };
    if let Some(session) = session { log.secret(&hex(&session)); proxy.activate(session); }
    // The added cards (#75): the game's card files with them, before the game loads its cards. A server that adds cards the
    // game can't be given must not reach the game at all: it would get rows it doesn't have.
    if session.is_some() {
        match yo3_launcher::added::fetch(proxy.ports.http, &hex(&proxy.cookie)).await {
            Ok(added) if added.is_empty() => {}
            Ok(added) => {
                let prepared = args.game_dir.as_deref().context("no game folder").and_then(|game| yo3_launcher::added::prepare(
                    &yo3_launcher::added::base_for(profile.parent().unwrap_or(std::path::Path::new("."))), game, &added,
                    args.anime_art.as_deref().or(args.game_dir.as_deref().map(|g| g.join(yo3_launcher::art::FOLDER)).as_deref())));
                let folder = match prepared {
                    Ok(done) => done.folder.inspect(|f| log.line(&format!("Added cards: {} card(s), files {} ({} written now)", added.cards.len(), f.display(), done.written))),
                    Err(error) => None.inspect(|_: &PathBuf| log.error(&format!("Added cards: the game's card files could not be extended: {error:#}"))),
                };
                let named: anyhow::Result<Option<String>> = match folder.as_deref() {
                    None => Ok(None),
                    Some(f) => match yo3_launcher::runtime::native_path(f) {
                        Err(e) => Err(e.into()),
                        #[cfg(windows)] Ok(native) => Ok(Some(native.display().to_string())),
                        #[cfg(not(windows))] Ok(native) => windows_path(&native, args.wine_prefix.as_deref()).await.map(Some),
                    },
                };
                match (named, folder.is_some()) {
                    (Ok(Some(path)), true) if yo3_launcher::added::point(&added_pointer, Some(&path)).is_ok() => {}
                    _ => {
                        let _ = child.kill().await;
                        anyhow::bail!("This server has cards your game doesn't have yet, and the launcher could not add them. Check that the game folder is complete and writable, then try again, or export logs for help.");
                    }
                }
            }
            Err(error) => {
                let _ = child.kill().await;
                anyhow::bail!("The server's added cards could not be read: {error:#}. Try again, or export logs for help.");
            }
        }
    }
    // The card profile the game loads (what discovery announces for it), kept to notice when the server pins another one.
    let mut loaded_profile: Option<String> = None;
    // The local decks follow the card profile the game is about to load (#60); nothing here can stop the game from starting.
    if let (Some(_), Some(original)) = (session, &args.client_data) {
        let folder = yo3_launcher::decks::folder_for(original, &proxy.server_id);
        match yo3_launcher::decks::fetch(proxy.ports.http, &hex(&proxy.cookie)).await {
            Ok(file) => {
                let wanted = yo3_launcher::decks::Wanted::parse(&file);
                loaded_profile = Some(wanted.announced());
                match yo3_launcher::decks::adapt(&folder, &wanted) {
                    Ok(done) if done.adapted > 0 || !done.skipped.is_empty() => log.line(&format!("Local decks and the card profile {}: {} deck(s) changed, {} card(s) moved, {} file(s) left alone", wanted.hash, done.adapted, done.moved, done.skipped.len())),
                    Ok(_) => {}
                    Err(error) => log.warn(&format!("Local decks were not adapted to the card profile: {error}")),
                }
            }
            Err(error) => log.warn(&format!("Local decks were not adapted: the card profile could not be fetched: {error:#}")),
        }
    }
    let game_log = log.clone();
    let output = tokio::spawn(async move { capture(stdout, game_log, true).await; });
    let result = async {
        child.stdin.as_mut().unwrap().write_all(b"resume\n").await?;
        child.stdin.take();
        write_ready(&args, &proxy)?;
        if session.is_some() { eprintln!("Authenticated with mod interface {} ready. Local gate {}, front {}, duel {}, HTTP {}.", yo3_tunnel::MOD_ABI, proxy.ports.gate, proxy.ports.front, proxy.ports.duel, proxy.ports.http); }
        log.line(if session.is_some() { "Authenticated; game resumed" } else { "Game resumed without a sign-in: its own login dialog" });
        // Delete Login Data in the game forgets this device's saved sign-in for the server (#33).
        let sign_outs = proxy.sign_outs();
        let forget = || async {
            match yo3_launcher::direct::sign_out(&endpoint, vault.as_deref(), &yo3_launcher::vault::SystemVault).await {
                Ok(true) => log.line("Delete Login Data: this device's saved sign-in is forgotten"),
                Ok(false) => {}
                Err(error) => log.error(&format!("Delete Login Data failed: {error:#}")),
            }
        };
        let mut typed = proxy.typed_logins().context("the login form's channel is gone")?;
        let mut launcher_gone = false;
        // The server's card profile is asked of discovery now and then: a host who changes it makes the running game
        // out of date, which the Desktop shows as an offer to restart (decks.rs, #60).
        let mut profile_tick = tokio::time::interval(std::time::Duration::from_secs(20));
        profile_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let status = loop {
            tokio::select! {
                _ = profile_tick.tick(), if loaded_profile.is_some() && args.ready_file.is_some() => {
                    if let (Some(loaded), Some(ready)) = (&loaded_profile, &args.ready_file)
                        && let Ok(Ok(info)) = tokio::time::timeout(std::time::Duration::from_secs(10), endpoint.info()).await
                        && yo3_launcher::decks::note_staleness(ready, loaded, &info.card_profile)
                    { log.line("The server pins another card profile than this game loaded: restart the game to see it") }
                }
                status = child.wait() => break Some(status?),
                result = &mut interrupt => { result?; child.kill().await?; break None }
                _ = sign_outs.notified() => forget().await,
                Some(login) = typed.recv() => {
                    // The game's own login form (#34): its next login waits for this.
                    match yo3_launcher::direct::form_sign_in(&endpoint, &proxy, &login, vault.as_deref(), &yo3_launcher::vault::SystemVault, &mut session, key.owns_session()).await {
                        Ok(()) => log.line(&format!("Signed in from the game's login form{}", if login.stay { "; this device stays signed in" } else { "" })),
                        Err(failure) => log.warn(&format!("The game's login form: refused ({:?}): {}", failure.refusal, failure.reason)),
                    }
                }
                stop = &mut desktop_stop, if !launcher_gone => match stop {
                    yo3_launcher::DesktopStop::Asked => { child.kill().await?; break None }
                    // The launcher closed or crashed while the game runs: keep playing; the session ends with the game.
                    yo3_launcher::DesktopStop::Gone => { log.line("The launcher went away; the game keeps running"); launcher_gone = true }
                }
            }
        };
        if let Some(status) = status { ensure!(status.success(), "Game or injector exited with an error ({status})"); }
        Ok(())
    }.await;
    // Before the sign-out, while the session can still file it: a game that broke sends its log to the server.
    if let Some(session) = session && !args.no_problem_reports && !yo3_launcher::problems::off(&profile) {
        let failure = result.as_ref().err().map(|e| format!("{e:#}"));
        yo3_launcher::problems::send_if_broken(&endpoint, &session, &log, failure.as_deref()).await;
    }
    if let Some(session) = session && key.owns_session() { let _ = endpoint.logout(&session).await; }
    if child.try_wait()?.is_none() { let _ = child.kill().await; }
    let _ = tokio::time::timeout(std::time::Duration::from_secs(1), output).await;
    let detail = tokio::time::timeout(std::time::Duration::from_secs(1), error_output).await.ok().and_then(Result::ok).unwrap_or_default();
    if detail.trim().is_empty() { result }
    else { result.with_context(|| yo3_tunnel::diagnostics::redact(&detail, &[key.secret(), hex(&proxy.cookie), session.map(|s| hex(&s)).unwrap_or_default()], None)) }
}

/// `--sign-in` and `--register`: a password sign-in for the Scripts edition (#23), kept in the credential store.
async fn sign_in(args: &Args) -> anyhow::Result<()> {
    use yo3_launcher::signin::{self, Kept};
    let register = args.register.is_some();
    let given = args.register.as_ref().or(args.recover.as_ref()).or(args.sign_in.as_ref()).expect("account mode");
    let (server, user) = (given[0].as_str(), given[1].trim());
    let profile = args.profile.as_deref().context("--profile names the file `play` will use")?;
    let invite = signin::server(server, args.identity.as_deref())?;
    let endpoint = signin::endpoint(&invite)?;
    let info = endpoint.info().await.with_context(|| format!("{} can't be reached at {}", invite.name, invite.address))?;
    info.ensure_compatible(yo3_tunnel::PROTOCOL_VERSION, yo3_tunnel::MOD_ABI)?;
    info.ensure_launcher(yo3_tunnel::BUILD_VERSION)?;
    let mut input = Input::new();
    let password = if args.recover.is_some() {
        let code = input.secret("Setup, recovery or reset code: ")?;
        let password = input.secret("Choose a password (8 to 16 letters or digits): ")?;
        anyhow::ensure!(input.secret("The password again: ")? == password, "The two passwords differ.");
        let recovery = signin::recover(&endpoint, user, &code, &password).await?;
        println!("Password set for {user} on {}.", invite.name);
        if let Some(code) = recovery { println!("Recovery code: {code}\nKeep it somewhere safe: it resets the password when you forget it."); }
        password
    } else if register {
        // A link made for one friend carries their invite code.
        let code = match &invite.code { Some(code) => { println!("Invite code {code}, from the link."); code.clone() }
            None => input.line("Invite code from the host (Enter if the server needs none): ")? };
        let password = input.secret("Choose a password (8 to 16 letters or digits): ")?;
        anyhow::ensure!(input.secret("The password again: ")? == password, "The two passwords differ.");
        eprintln!("The rules are in docs/rules-privacy.md (in this folder), plus the host's own announcements.");
        let rules = input.line("Do you accept this server's rules? Type yes: ")?.eq_ignore_ascii_case("yes");
        anyhow::ensure!(rules, "Accept the server's rules to make an account.");
        let recovery = signin::register(&endpoint, user, &password, &code, rules).await?;
        println!("Account {user} made on {}.\nRecovery code: {recovery}\nKeep it somewhere safe: it resets the password when you forget it.", invite.name);
        password
    } else {
        input.secret(&format!("Password for {user} on {}: ", invite.name))?
    };
    let kept = signin::sign_in(&endpoint, &invite, user, &password, &yo3_launcher::vault::SystemVault).await?;
    signin::write_profile(profile, &invite)?;
    match kept {
        Kept::Saved => println!("Signed in as {user}: this PC stays signed in to {} (in the system's credential store).", invite.name),
        Kept::NotKept(why) => {
            println!("Signed in as {user}, but this PC can't stay signed in.");
            eprintln!("The system's credential store isn't available ({why}). The game's own login form will ask for your User ID and password at each start.");
        }
    }
    println!("Play with: play --profile \"{}\" --game-dir GAME_FOLDER", std::path::absolute(profile).unwrap_or(profile.to_path_buf()).display());
    Ok(())
}

/// What the player types: a terminal's lines, with the echo off for a password; a pipe's lines otherwise.
struct Input { lines: std::io::Lines<std::io::StdinLock<'static>> }

impl Input {
    fn new() -> Self { use std::io::BufRead; Self { lines: std::io::stdin().lock().lines() } }
    fn line(&mut self, prompt: &str) -> anyhow::Result<String> {
        use std::io::Write;
        eprint!("{prompt}"); std::io::stderr().flush()?;
        Ok(self.lines.next().context("no answer on standard input")??.trim().to_string())
    }
    fn secret(&mut self, prompt: &str) -> anyhow::Result<String> {
        use std::io::IsTerminal;
        let echo = std::io::stdin().is_terminal().then(echo_off).flatten();
        let read = self.line(prompt);
        // The terminal ends the hidden line itself (ECHONL); Windows' console doesn't.
        if let Some(restore) = echo { restore(); if cfg!(windows) { eprintln!() } }
        let secret = read?;
        anyhow::ensure!(!secret.is_empty(), "no password given");
        Ok(secret)
    }
}

/// Turns the terminal's echo off; the returned function turns it back on.
#[cfg(unix)]
fn echo_off() -> Option<Box<dyn FnOnce()>> {
    let mut before: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut before) } != 0 { return None }
    let mut quiet = before; quiet.c_lflag &= !libc::ECHO; quiet.c_lflag |= libc::ECHONL;
    if unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &quiet) } != 0 { return None }
    Some(Box::new(move || unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &before); }))
}
#[cfg(windows)]
fn echo_off() -> Option<Box<dyn FnOnce()>> {
    use windows_sys::Win32::System::Console::{GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_ECHO_INPUT, STD_INPUT_HANDLE};
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut before = 0;
    if unsafe { GetConsoleMode(handle, &mut before) } == 0 { return None }
    if unsafe { SetConsoleMode(handle, before & !ENABLE_ECHO_INPUT) } == 0 { return None }
    let handle = handle as usize;
    Some(Box::new(move || unsafe { SetConsoleMode(handle as _, before); }))
}

fn vault(args: &Args) -> anyhow::Result<()> {
    use yo3_launcher::vault::{SystemVault, Vault};
    if let Some(account) = &args.vault_delete { return SystemVault.delete(account) }
    let account = args.vault_set.as_deref().context("no vault entry named")?;
    use std::io::Read; let mut token = String::new(); std::io::stdin().take(4096).read_to_string(&mut token)?;
    let token = token.trim();
    ensure!(!token.is_empty() && token.bytes().all(|b| b.is_ascii_graphic()), "no device token on standard input");
    SystemVault.set(account, token)
}

async fn capture(mut source: impl tokio::io::AsyncRead + Unpin, log: yo3_tunnel::diagnostics::Session, client_trace: bool) -> String {
    use tokio::io::AsyncReadExt;
    let mut buffer = [0; 4096]; let mut writer = if client_trace { log.client_writer() } else { log.writer() }; let mut tail = Vec::new(); let mut truncated = false;
    loop {
        let n=match source.read(&mut buffer).await {Ok(n)=>n,Err(error)=>{log.error(&format!("Could not read game output: {error}"));break}};
        if n == 0 { break }
        let _ = std::io::Write::write_all(&mut writer, &buffer[..n]);
        tail.extend_from_slice(&buffer[..n]);
        if tail.len() > 8192 { tail.drain(..tail.len()-8192); truncated = true; }
    }
    // Discard a cut first line: a cropped credential suffix would evade whole-secret redaction.
    if truncated { let end = tail.iter().position(|b| *b == b'\n').map_or(tail.len(), |n| n+1); tail.drain(..end); }
    String::from_utf8_lossy(&tail).into_owned()
}

/// A survey report the launcher chose the place of itself is removed with the run.
struct SurveyCleanup(PathBuf);
impl Drop for SurveyCleanup { fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); } }

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

#[cfg(test)]
mod capture_tests {
    use super::*;
    #[tokio::test]
    async fn normal_launch_keeps_client_output_without_an_opt_in_and_scrubs_secrets() {
        let root=std::env::temp_dir().join(format!("yo3-client-output-{}",std::process::id())); let _=std::fs::remove_dir_all(&root);
        let log=yo3_tunnel::diagnostics::Session::start(&root,"game",None).unwrap();
        let _args=Args::try_parse_from(["yo3-launcher","--profile","profile.json","--game-dir","game"]).unwrap();
        capture(&b"client effect failed\nkey=abcdEFGH2345ijkmNPQR\n"[..],log,true).await;
        let text=yo3_tunnel::diagnostics::records(&root).unwrap().into_iter().filter(|r|r.name.ends_with(".log")).map(|r|std::fs::read_to_string(root.join(r.name)).unwrap()).collect::<String>();
        assert!(text.contains("client effect failed"),"normal launches must retain evidence: {text}");
        assert!(!text.contains("abcdEFGH2345ijkmNPQR"));
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, windows))]
mod native_permission_tests {
    #[test]
    fn only_the_game_process_tree_inherits_the_invoking_users_permission_level() {
        use std::{ffi::{OsStr, OsString}, path::Path};
        let before = std::env::var_os("__COMPAT_LAYER");
        let command = super::windows_injector(Path::new("injector.exe"), Path::new("game with spaces/yo3.exe"), Path::new("auth.dll"), Some(OsString::from("HIGHDPIAWARE")));
        let env = command.as_std().get_envs().find(|(key,_)| *key == OsStr::new("__COMPAT_LAYER")).and_then(|(_,value)| value);
        assert_eq!(env, Some(OsStr::new("HIGHDPIAWARE RunAsInvoker")));
        assert_eq!(std::env::var_os("__COMPAT_LAYER"), before, "launcher/server environment must stay unchanged");
        assert_eq!(command.as_std().get_args().collect::<Vec<_>>(), [OsStr::new("--control-stdin"), OsStr::new("game with spaces/yo3.exe"), OsStr::new("auth.dll")]);
    }
}
