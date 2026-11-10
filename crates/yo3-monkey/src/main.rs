//! yo3-monkey: protocol monkey for yo3-server. See "Monkey testing" in docs/roadmap.md and AGENTS.md for how to run it.
//!
//!   yo3-monkey fuzz   --seed 1 --plans 2000      plans one after another, the server checked after each
//!   yo3-monkey soak   --secs 300 --conns 24 ...  concurrent plans, duel pairs, floods, a canary and a monitor
//!   yo3-monkey duel   --seed 1 --pairs 10        Auto Duel between two scripted clients, then random duel commands
//!   yo3-monkey repro  FILE                       replay a saved plan against a fresh server
//!   yo3-monkey minimize FILE                     cut a failing plan down to the steps that matter
mod plans;
mod run;
mod wire;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use run::*;
use wire::{Corpus, Ports, Target};

#[derive(Parser)]
struct Args {
    /// Where the server's data dir, packet log, server.log, stats.csv and failures/ go (never the repo's data/).
    #[arg(long, global = true)]
    scratch: Option<PathBuf>,
    /// The server binary to start and supervise (restarted when it dies).
    #[arg(long, global = true, default_value = "target/release/yo3-server")]
    server_bin: PathBuf,
    /// Extra arguments for the server, e.g. `--server-arg=--duel-lp --server-arg=1000`.
    #[arg(long, global = true)]
    server_arg: Vec<String>,
    /// The packet logs of the real client, the corpus of requests.
    #[arg(long, global = true, default_value = "logs")]
    corpus: PathBuf,
    #[arg(long, global = true, default_value_t = 400)]
    corpus_files: usize,
    #[arg(long, global = true, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, global = true, default_value_t = 5730)]
    gate_port: u16,
    #[arg(long, global = true, default_value_t = 1213)]
    front_port: u16,
    #[arg(long, global = true, default_value_t = 1214)]
    duel_port: u16,
    #[arg(long, global = true, default_value_t = 8080)]
    http_port: u16,
    /// Duel and soak runs start the server with `--duel-check`, so every duel's client view is compared with the rules
    /// engine after each step (a difference is an ERROR, so a failure); this turns that off.
    #[arg(long, global = true)]
    no_duel_check: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Fuzz {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 1000)]
        plans: u64,
        /// Only this port: gate, front, duel or http.
        #[arg(long)]
        only: Option<String>,
        /// Plans run at once (the server is checked after each batch).
        #[arg(long, default_value_t = 4)]
        par: u64,
    },
    Soak {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 120)]
        secs: u64,
        /// Concurrent fuzzing connections.
        #[arg(long, default_value_t = 16)]
        conns: usize,
        /// Concurrent Auto Duel pairs.
        #[arg(long, default_value_t = 2)]
        duel_pairs: usize,
        /// A canary round trip slower than this is a stall (milliseconds).
        #[arg(long, default_value_t = 2000)]
        stall_ms: u64,
        /// Kill -9 the server every this many seconds (0: never) to see what a crash leaves in the data dir.
        #[arg(long, default_value_t = 0)]
        kill_every: u64,
    },
    Duel {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 10)]
        pairs: u64,
    },
    /// Connects from many different local addresses (127.x.y.z): the server keeps one account per client address, so
    /// this is what the accounts file does when new addresses keep arriving.
    Addrs {
        #[arg(long, default_value_t = 2000)]
        n: u32,
    },
    Repro {
        file: PathBuf,
    },
    Minimize {
        file: PathBuf,
        /// A substring of the trouble to look for (default: any panic or ERROR, or the server dying); a log line with it
        /// counts as trouble, whatever its level.
        #[arg(long)]
        sig: Option<String>,
    },
}

struct Env {
    /// The monkey killed the server itself (crash recovery): its death is no failure, what it leaves behind is checked.
    killed: std::sync::atomic::AtomicBool,
    ports: Ports,
    corpus: Arc<Corpus>,
    sup: std::sync::Mutex<Supervisor>,
    log: std::sync::Mutex<LogWatch>,
    reporter: Reporter,
    scratch: PathBuf,
}

impl Env {
    /// Restarts the server if it died; true when it did.
    async fn revive(&self, plans: &[Arc<plans::Plan>]) -> bool {
        let died = lock(&self.sup).exited();
        let Some(status) = died else { return false };
        let expected = self.killed.swap(false, Ordering::Relaxed);
        let tail = lock(&self.log).tail(40);
        if !expected {
            self.reporter.fail("server died", &format!("exit status {status}"), plans, &tail);
        }
        let _ = lock(&self.log).trouble();
        let started = lock(&self.sup).start();
        if started.is_ok() {
            let ready = lock(&self.sup).wait_ready_handle();
            ready.await;
        }
        if expected {
            tokio::time::sleep(Duration::from_millis(500)).await;
            self.look(plans);
            self.check_data();
        }
        true
    }

    /// A data file that did not parse after a restart is renamed `NAME.corrupt-<time>` by the server.
    fn check_data(&self) {
        let dir = lock(&self.sup).data_dir();
        let Ok(rd) = std::fs::read_dir(&dir) else { return };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.contains(".corrupt-") {
                self.reporter.fail("data corrupted by a crash", &format!("{name} in the data dir after a restart"), &[], &lock(&self.log).tail(20));
                let _ = std::fs::remove_file(e.path());
            }
        }
    }

    /// Panics and ERRORs since the last look, reported with `plans`.
    fn look(&self, plans: &[Arc<plans::Plan>]) -> usize {
        let lines = lock(&self.log).trouble();
        for l in &lines {
            let tail = lock(&self.log).tail(30);
            self.reporter.fail("server log", l, plans, &tail);
        }
        lines.len()
    }
}

impl Supervisor {
    fn wait_ready_handle(&self) -> impl std::future::Future<Output = bool> + use<> {
        let ports = self_ports(self);
        async move {
            for _ in 0..200 {
                if wire::Conn::open(&ports.addr(Target::Gate)).await.is_ok() {
                    return true;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            false
        }
    }
}

fn self_ports(s: &Supervisor) -> Ports {
    s.ports().clone()
}

/// The ids of the first two players of the account at 127.0.0.`from` (Select Player's list, 0x2260); the account is
/// made by this first contact, so the accounts are asked for in order.
async fn discover_players(ports: &Ports, from: u8) -> Option<[u32; 2]> {
    let ip = std::net::IpAddr::from([127, 0, 0, from]);
    let mut c = wire::Conn::open_from(&ports.addr(Target::Front), Some(ip)).await.ok()?;
    c.send_version().await.ok()?;
    c.send(&wire::req(0x1F49)).await.ok()?;
    let (list, _) = c.expect(0x2260, Duration::from_secs(5)).await;
    let ids: Vec<u32> = list?.items.iter().filter_map(|i| i.get("player_id").and_then(yo3_proto::Value::as_u32)).collect();
    Some([*ids.first()?, *ids.get(1).or(ids.first())?])
}

async fn setup(a: &Args) -> anyhow::Result<Arc<Env>> {
    let scratch = a.scratch.clone().ok_or_else(|| anyhow::anyhow!("--scratch DIR is required (a scratch directory, never the repo's data/)"))?;
    anyhow::ensure!(
        scratch.canonicalize().ok() != PathBuf::from("data").canonicalize().ok() || !scratch.exists(),
        "--scratch must not be the repo's data/"
    );
    std::fs::create_dir_all(&scratch)?;
    let mut corpus = Corpus::load(&a.corpus, a.corpus_files);
    println!(
        "corpus: {} sessions, {} request ids, {} field names, deck update: {}",
        corpus.sessions.len(),
        corpus.ids.len(),
        corpus.names.len(),
        corpus.deck_update.is_some()
    );
    let ports = Ports { host: a.host.clone(), gate: a.gate_port, front: a.front_port, duel: a.duel_port, http: a.http_port, pids: [1001, 1002] };
    let mut ports = ports;
    let cwd = std::env::current_dir()?;
    let mut server_args = a.server_arg.clone();
    if matches!(a.cmd, Cmd::Duel { .. } | Cmd::Soak { .. }) && !a.no_duel_check && !server_args.iter().any(|s| s == "--duel-check") {
        server_args.push("--duel-check".into());
    }
    let mut sup = Supervisor::new(a.server_bin.clone(), cwd, scratch.clone(), ports.clone(), server_args);
    sup.start()?;
    anyhow::ensure!(sup.wait_ready().await, "the server did not come up (see {})", sup.log_path().display());
    let mut accounts = vec![];
    for from in 1..=4u8 {
        accounts.extend(discover_players(&ports, from).await);
    }
    ports.pids = accounts.first().copied().unwrap_or(ports.pids);
    corpus.accounts = accounts;
    let log = LogWatch::new(sup.log_path());
    println!("server pid {} (scratch {})", sup.pid().unwrap_or(0), scratch.display());
    Ok(Arc::new(Env {
        ports,
        corpus: Arc::new(corpus),
        sup: std::sync::Mutex::new(sup),
        log: std::sync::Mutex::new(log),
        killed: std::sync::atomic::AtomicBool::new(false),
        reporter: Reporter::new(scratch.join("failures")),
        scratch,
    }))
}

fn stats_line(env: &Env) -> Sample {
    let sup = lock(&env.sup);
    sup.pid().map(|p| sample(p, &sup.data_dir(), &sup.packet_dir())).unwrap_or_default()
}

fn print_sample(label: &str, s: &Sample) {
    println!(
        "{label}: rss {} MB, {} fds, {} threads, data dir {} KB, packet log {} KB",
        s.rss_kb / 1024,
        s.fds,
        s.threads,
        s.data_bytes / 1024,
        s.log_bytes / 1024
    );
}

async fn finish(env: &Env, base: Sample, canary: &CanaryStats) -> usize {
    tokio::time::sleep(Duration::from_secs(4)).await;
    let end = stats_line(env);
    print_sample("server at start", &base);
    print_sample("server at end  ", &end);
    let tail = lock(&env.log).tail(30);
    if end.fds > base.fds + 40 {
        env.reporter.fail("fd growth", &format!("{} fds at start, {} at the end with every monkey connection closed", base.fds, end.fds), &[], &tail);
    }
    if end.rss_kb > base.rss_kb + 400 * 1024 {
        env.reporter.fail("memory growth", &format!("rss {} MB at start, {} MB at the end", base.rss_kb / 1024, end.rss_kb / 1024), &[], &tail);
    }
    if end.data_bytes > base.data_bytes + 64 * 1024 * 1024 {
        env.reporter.fail("data growth", &format!("data dir {} KB at start, {} KB at the end", base.data_bytes / 1024, end.data_bytes / 1024), &[], &tail);
    }
    match canary_round(&env.ports, env.ports.pids[1]).await {
        Ok(d) => println!("final canary round: {d:?}"),
        Err(e) => env.reporter.fail("canary", &format!("final round: {e}"), &[], &tail),
    }
    // A clean restart on what the run left in the data dir (and the server's own graceful stop first).
    lock(&env.sup).stop();
    let _ = lock(&env.log).trouble();
    if lock(&env.sup).start().is_ok() {
        let ready = lock(&env.sup).wait_ready_handle();
        if ready.await {
            tokio::time::sleep(Duration::from_millis(500)).await;
            env.look(&[]);
            env.check_data();
            match canary_round(&env.ports, env.ports.pids[1]).await {
                Ok(d) => println!("restart on the run's data dir: up, canary round {d:?}"),
                Err(e) => env.reporter.fail("restart", &format!("after a graceful stop, on the run's data: {e}"), &[], &lock(&env.log).tail(20)),
            }
        } else {
            env.reporter.fail("restart", "the server did not come up again on the run's data dir", &[], &lock(&env.log).tail(30));
        }
    }
    println!(
        "canary: {} rounds, {} failed, {} stalls; p50 {} ms, p99 {} ms, max {} ms",
        canary.rounds.load(Ordering::Relaxed),
        canary.failures.load(Ordering::Relaxed),
        canary.spikes.load(Ordering::Relaxed),
        canary.percentile(0.5),
        canary.percentile(0.99),
        canary.percentile(1.0)
    );
    let warns = lock(&env.log).warns.clone();
    let mut warns: Vec<_> = warns.into_iter().collect();
    warns.sort_by_key(|b| std::cmp::Reverse(b.1));
    println!("server WARN lines (expected: closes, unhandled ids, refusals), top kinds:");
    for (k, n) in warns.iter().take(8) {
        println!("  {n:>7}  {k}");
    }
    println!("failures: {} ({} distinct)", env.reporter.total(), env.reporter.signatures().len());
    for (sig, n) in env.reporter.signatures() {
        println!("  {n:>5}x {sig}");
    }
    env.reporter.total()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let a = Args::parse();
    let env = setup(&a).await?;
    let started = Instant::now();
    let canary = Arc::new(CanaryStats::default());
    let base = stats_line(&env);
    let failures = match a.cmd {
        Cmd::Fuzz { seed, plans, only, par } => {
            let only = only.as_deref().and_then(Target::parse);
            let recent = Recent::default();
            let mut messages = 0usize;
            let mut slow: Vec<(Duration, u64, &str)> = vec![];
            let mut i = 0u64;
            while i < plans {
                // `par` plans at once; a failure is attributed to the whole batch (minimize cuts it down).
                let batch: Vec<Arc<plans::Plan>> = (i..(i + par).min(plans)).map(|k| Arc::new(plans::plan(seed + k, &env.corpus, only))).collect();
                let runs: Vec<_> = batch
                    .iter()
                    .map(|plan| {
                        let (ports, plan) = (env.ports.clone(), plan.clone());
                        tokio::spawn(async move {
                            let t0 = Instant::now();
                            let out = run_plan(&ports, &plan).await;
                            (t0.elapsed(), out.sent, plan.seed, plan.target.name())
                        })
                    })
                    .collect();
                for r in runs {
                    if let Ok((d, sent, s, name)) = r.await {
                        slow.push((d, s, name));
                        messages += sent;
                    }
                }
                for p in &batch {
                    recent.note(p.clone());
                }
                let first = i;
                i += batch.len() as u64;
                tokio::time::sleep(Duration::from_millis(15)).await;
                if env.revive(&batch).await {
                    continue;
                }
                env.look(&batch);
                if i / 20 != first / 20 {
                    let t = Instant::now();
                    match canary_round(&env.ports, env.ports.pids[1]).await {
                        Ok(d) => {
                            canary.record(d.as_millis() as u32);
                            if d > Duration::from_millis(2000) {
                                canary.spikes.fetch_add(1, Ordering::Relaxed);
                                env.reporter.fail("stall", &format!("canary round took {d:?} in the batch from seed {}", seed + first), &batch, "");
                            }
                        }
                        Err(e) => {
                            canary.failures.fetch_add(1, Ordering::Relaxed);
                            env.reporter.fail("canary", &format!("{e} (after {:?}) in the batch from seed {}", t.elapsed(), seed + first), &batch, &lock(&env.log).tail(20));
                        }
                    }
                }
                if i / 500 != first / 500 {
                    print_sample(&format!("after {i} plans"), &stats_line(&env));
                }
            }
            slow.sort_by_key(|b| std::cmp::Reverse(b.0));
            println!("slowest plans: {:?}", &slow[..slow.len().min(5)]);
            println!("fuzz: seeds {seed}..{} = {plans} plans, {messages} messages in {:?}", seed + plans - 1, started.elapsed());
            finish(&env, base, &canary).await
        }
        Cmd::Duel { seed, pairs } => {
            let stats = Arc::new(DuelStats::default());
            for i in 0..pairs {
                let (a, b) = duel_pair(&env.ports, &env.corpus, &stats, seed + i).await;
                println!("pair seed {}: stages {a} and {b}", seed + i);
                tokio::time::sleep(Duration::from_millis(200)).await;
                if env.revive(&[]).await {
                    continue;
                }
                env.look(&[]);
                if let Err(e) = canary_round(&env.ports, env.ports.pids[1]).await {
                    env.reporter.fail("canary", &format!("after duel pair seed {}: {e}", seed + i), &[], &lock(&env.log).tail(20));
                }
            }
            print_duel(&stats);
            finish(&env, base, &canary).await
        }
        Cmd::Addrs { n } => {
            let mut slowest = Duration::ZERO;
            for i in 0..n {
                let ip = std::net::Ipv4Addr::new(127, 1 + (i / 65_000) as u8, ((i / 250) % 250) as u8, (i % 250) as u8 + 1);
                let t = Instant::now();
                let ok = async {
                    let mut c = wire::Conn::open_from(&env.ports.addr(Target::Front), Some(ip.into())).await?;
                    c.send_version().await?;
                    Ok::<_, std::io::Error>(c.expect(0x219F, Duration::from_secs(10)).await.0.is_some())
                }
                .await;
                slowest = slowest.max(t.elapsed());
                if !matches!(ok, Ok(true)) {
                    env.reporter.fail("addrs", &format!("address {ip} (#{i}): {ok:?}"), &[], &lock(&env.log).tail(10));
                }
                if i % 500 == 499 {
                    print_sample(&format!("after {} addresses (slowest connect so far {slowest:?})", i + 1), &stats_line(&env));
                }
            }
            finish(&env, base, &canary).await
        }
        Cmd::Soak { seed, secs, conns, duel_pairs, stall_ms, kill_every } => soak(&env, base, canary.clone(), seed, secs, conns, duel_pairs, stall_ms, kill_every).await,
        Cmd::Repro { file } => {
            let plan = plans::Plan::from_text(&std::fs::read_to_string(&file)?).ok_or_else(|| anyhow::anyhow!("not a plan file"))?;
            let plan = Arc::new(plan);
            let out = run_plan(&env.ports, &plan).await;
            println!("sent {} steps, received {} messages, io: {:?}", out.sent, out.received, out.error);
            tokio::time::sleep(Duration::from_millis(500)).await;
            env.revive(std::slice::from_ref(&plan)).await;
            env.look(std::slice::from_ref(&plan));
            finish(&env, base, &canary).await
        }
        Cmd::Minimize { file, sig } => {
            let plan = plans::Plan::from_text(&std::fs::read_to_string(&file)?).ok_or_else(|| anyhow::anyhow!("not a plan file"))?;
            lock(&env.log).also = sig.clone();
            minimize(&env, plan, sig, &file).await?;
            0
        }
    };
    lock(&env.sup).stop();
    if failures > 0 {
        println!("{failures} failure(s); see {}/failures", env.scratch.display());
    }
    std::process::exit(if failures > 0 { 1 } else { 0 });
}

fn print_duel(s: &DuelStats) {
    let g = |a: &std::sync::atomic::AtomicUsize| a.load(Ordering::Relaxed);
    println!(
        "duels: {} pairs; {} matched, {} entered the duel connection, {} janken, {} began, {} ended with a result; {} random actions sent, {} left early",
        g(&s.pairs),
        g(&s.matched),
        g(&s.entered),
        g(&s.janken),
        g(&s.began),
        g(&s.ended),
        g(&s.actions),
        g(&s.left_early)
    );
}

#[allow(clippy::too_many_arguments)]
async fn soak(env: &Arc<Env>, base: Sample, canary: Arc<CanaryStats>, seed: u64, secs: u64, conns: usize, duel_pairs: usize, stall_ms: u64, kill_every: u64) -> usize {
    let stop = stop_flag();
    let recent = Arc::new(Recent::default());
    let duel_stats = Arc::new(DuelStats::default());
    let counter = Arc::new(std::sync::atomic::AtomicU64::new(seed));
    let mut tasks = vec![];
    for _ in 0..conns {
        let (env, stop, recent, counter) = (env.clone(), stop.clone(), recent.clone(), counter.clone());
        tasks.push(tokio::spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                let s = counter.fetch_add(1, Ordering::Relaxed);
                let plan = Arc::new(plans::plan(s, &env.corpus, None));
                recent.note(plan.clone());
                let _ = run_plan(&env.ports, &plan).await;
            }
        }));
    }
    for k in 0..duel_pairs {
        let (env, stop, stats) = (env.clone(), stop.clone(), duel_stats.clone());
        tasks.push(tokio::spawn(async move {
            let mut i = 0u64;
            while !stop.load(Ordering::Relaxed) {
                duel_pair(&env.ports, &env.corpus, &stats, seed.wrapping_mul(1000) + k as u64 * 100_000 + i).await;
                i += 1;
            }
        }));
    }
    // The canary.
    {
        let (env, stop, canary, recent) = (env.clone(), stop.clone(), canary.clone(), recent.clone());
        tasks.push(tokio::spawn(async move {
            while !stop.load(Ordering::Relaxed) {
                let t = Instant::now();
                match canary_round(&env.ports, env.ports.pids[1]).await {
                    Ok(d) => {
                        canary.record(d.as_millis() as u32);
                        if d > Duration::from_millis(stall_ms) {
                            canary.spikes.fetch_add(1, Ordering::Relaxed);
                            env.reporter.fail("stall", &format!("canary round took {d:?}"), &recent.since(5, 40), &lock(&env.log).tail(10));
                        }
                    }
                    Err(e) => {
                        canary.failures.fetch_add(1, Ordering::Relaxed);
                        env.reporter.fail("canary", &format!("{e} (after {:?})", t.elapsed()), &recent.since(10, 40), &lock(&env.log).tail(20));
                    }
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }));
    }
    // Floods every so often.
    {
        let (env, stop) = (env.clone(), stop.clone());
        tasks.push(tokio::spawn(async move {
            let mut i = 0u64;
            while !stop.load(Ordering::Relaxed) {
                tokio::time::sleep(Duration::from_secs(12)).await;
                let t = *wire_pick(i);
                flood(&env.ports, t, 400, Duration::from_secs(2), seed + i).await;
                i += 1;
            }
        }));
    }
    // The monitor: the server alive, the log, and resource use every two seconds.
    let csv = env.scratch.join("stats.csv");
    let mut rows = String::from("secs,rss_kb,fds,threads,data_bytes,log_bytes,plans,canary_p99_ms\n");
    let end = started_plus(secs);
    let (mut max, mut last_data) = (Sample::default(), base.data_bytes);
    let (mut last_kill, mut kills) = (Instant::now(), 0u32);
    while Instant::now() < end {
        tokio::time::sleep(Duration::from_secs(2)).await;
        if kill_every > 0 && last_kill.elapsed() > Duration::from_secs(kill_every) {
            last_kill = Instant::now();
            env.killed.store(true, Ordering::Relaxed);
            if let Some(pid) = lock(&env.sup).pid() {
                let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).status();
            }
            kills += 1;
        }
        let ctx = recent.since(5, 40);
        if env.revive(&ctx).await {
            continue;
        }
        env.look(&ctx);
        let s = stats_line(env);
        max.rss_kb = max.rss_kb.max(s.rss_kb);
        max.fds = max.fds.max(s.fds);
        max.threads = max.threads.max(s.threads);
        rows += &format!(
            "{},{},{},{},{},{},{},{}\n",
            (secs as i64 - end.saturating_duration_since(Instant::now()).as_secs() as i64),
            s.rss_kb,
            s.fds,
            s.threads,
            s.data_bytes,
            s.log_bytes,
            recent.plans.load(Ordering::Relaxed),
            canary.percentile(0.99)
        );
        if s.data_bytes > last_data + 32 * 1024 * 1024 {
            env.reporter.fail("data growth", &format!("data dir grew from {} KB to {} KB in two seconds", last_data / 1024, s.data_bytes / 1024), &ctx, "");
        }
        last_data = s.data_bytes;
        if s.fds > 3000 || s.rss_kb > 4 * 1024 * 1024 {
            env.reporter.fail("resource limit", &format!("{} fds, rss {} MB", s.fds, s.rss_kb / 1024), &ctx, "");
        }
    }
    stop.store(true, Ordering::Relaxed);
    for t in tasks {
        let _ = tokio::time::timeout(Duration::from_secs(60), t).await;
    }
    let _ = std::fs::write(csv, rows);
    println!(
        "soak: {secs} s, {conns} fuzzing connections, {duel_pairs} duel pairs: {} plans, {} messages",
        recent.plans.load(Ordering::Relaxed),
        recent.messages.load(Ordering::Relaxed)
    );
    println!("peak: rss {} MB, {} fds, {} threads", max.rss_kb / 1024, max.fds, max.threads);
    print_duel(&duel_stats);
    if kills > 0 {
        println!("the server was killed (SIGKILL) {kills} times and restarted on the same data dir");
    }
    finish(env, base, &canary).await
}

fn started_plus(secs: u64) -> Instant {
    Instant::now() + Duration::from_secs(secs)
}

fn wire_pick(i: u64) -> &'static Target {
    [Target::Front, Target::Gate, Target::Http, Target::Duel][(i % 4) as usize..].first().unwrap()
}

/// Delta debugging on a plan's steps: removes chunks while the trouble stays. A trial that kills the server restarts it.
async fn minimize(env: &Arc<Env>, plan: plans::Plan, sig: Option<String>, file: &std::path::Path) -> anyhow::Result<()> {
    let trouble = |env: &Arc<Env>, plan: Arc<plans::Plan>| {
        let env = env.clone();
        async move {
            let _ = run_plan(&env.ports, &plan).await;
            tokio::time::sleep(Duration::from_millis(300)).await;
            let died = lock(&env.sup).exited();
            if let Some(s) = died {
                lock(&env.sup).start().ok();
                let ready = lock(&env.sup).wait_ready_handle();
                ready.await;
                let _ = lock(&env.log).trouble();
                return Some(format!("server died: {s}"));
            }
            lock(&env.log).trouble().into_iter().next()
        }
    };
    let first = trouble(env, Arc::new(plan.clone())).await;
    let Some(found) = first else {
        println!("the plan shows no trouble (a panic, an ERROR or the server dying); nothing to minimize");
        return Ok(());
    };
    let want = sig.unwrap_or_else(|| plans::signature_of(&found));
    println!("trouble: {found}\nlooking for: {want:?}");
    let mut steps = plan.steps.clone();
    let mut chunk = (steps.len() / 2).max(1);
    let mut trials = 0;
    while chunk >= 1 && trials < 400 {
        let mut i = 0;
        let mut shrunk = false;
        while i < steps.len() {
            let mut cand = steps.clone();
            cand.drain(i..(i + chunk).min(steps.len()));
            trials += 1;
            let p = Arc::new(plans::Plan { seed: plan.seed, from: plan.from, target: plan.target, steps: cand.clone() });
            if trouble(env, p).await.is_some_and(|t| t.contains(&want)) {
                steps = cand;
                shrunk = true;
            } else {
                i += chunk;
            }
        }
        if !shrunk {
            if chunk == 1 {
                break;
            }
            chunk = (chunk / 2).max(1);
        }
    }
    let out = file.with_extension("min.txt");
    let min = plans::Plan { seed: plan.seed, from: plan.from, target: plan.target, steps };
    std::fs::write(&out, min.to_text())?;
    println!("{} steps -> {} steps in {trials} trials: {}", plan.steps.len(), min.steps.len(), out.display());
    lock(&env.sup).stop();
    Ok(())
}
