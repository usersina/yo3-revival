//! A downloaded version becomes active only after a live process acknowledges its first window.
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use anyhow::{ensure, Context};
use serde::{Serialize, Deserialize};
use super::install::{self, Installed};
#[derive(Clone)]
pub struct Readiness { path: PathBuf, nonce: String }
#[derive(Serialize, Deserialize)]
struct Ack { nonce: String, version: String, pid: u32 }
impl Readiness {
    pub fn new(root: &Path, path: PathBuf, nonce: String) -> anyhow::Result<Self> {
        ensure!(nonce.len() == 64 && nonce.bytes().all(|b| b.is_ascii_hexdigit()), "Invalid update readiness nonce");
        ensure!(path.file_name().is_some_and(|n| n == "ready.json") && !path.exists(), "Invalid update readiness file");
        let jobs = root.canonicalize()?.join("updates/jobs"); let parent = path.parent().context("Missing update job")?;
        ensure!(parent.parent() == Some(jobs.as_path()) && parent.canonicalize()? == parent && !std::fs::symlink_metadata(parent)?.file_type().is_symlink(), "Update readiness must stay in private job storage");
        Ok(Self { path, nonce })
    }
    pub fn write(&self, version: &str) -> anyhow::Result<()> {
        use std::io::Write;
        yo3_tunnel::diagnostics::private_create(&self.path)?.write_all(&serde_json::to_vec(&Ack { nonce: self.nonce.clone(), version: version.into(), pid: std::process::id() })?)?;
        Ok(())
    }
}
#[derive(Clone, Default)]
pub struct Startup { pub readiness: Option<Readiness>, pub resume_hosting: bool }
impl Startup {
    pub fn parse(root: &Path, args: impl IntoIterator<Item=String>) -> anyhow::Result<Self> {
        let mut args = args.into_iter(); let (mut path, mut nonce) = (None, None); let mut resume_hosting = false;
        while let Some(arg) = args.next() { match arg.as_str() {
            "--update-ready-file" if path.is_none() => path = Some(PathBuf::from(args.next().context("Missing readiness path")?)),
            "--update-nonce" if nonce.is_none() => nonce = Some(args.next().context("Missing readiness nonce")?),
            "--resume-hosting" if !resume_hosting => resume_hosting = true,
            _ => anyhow::bail!("Unknown launcher argument"),
        } }
        let readiness = match (path, nonce) { (None, None) => None, (Some(path), Some(nonce)) => Some(Readiness::new(root, path, nonce)?), _ => anyhow::bail!("Incomplete update readiness request") };
        ensure!(!resume_hosting || readiness.is_some(), "Hosting can resume only during a verified update restart");
        Ok(Self { readiness, resume_hosting })
    }
}
pub fn target(root: &Path, current: &str, executable: &Path) -> anyhow::Result<Option<Installed>> {
    let Ok(current) = super::version(current) else { return Ok(None) };
    if !current.pre.is_empty() { return Ok(None) }
    let Some(installed) = install::active(root)? else { return Ok(None) };
    let candidate = super::version(&installed.tag)?;
    if candidate.cmp_precedence(&current).is_le() || executable.canonicalize().ok() == installed.executable().canonicalize().ok() { return Ok(None) }
    install::validate(root, &installed)?;
    ensure!(installed.platform == super::Platform::current().name().context("Unsupported update platform")?, "Installed update is for a different platform");
    Ok(Some(installed))
}
struct Candidate(Option<Child>);
impl Drop for Candidate { fn drop(&mut self) { if let Some(child) = &mut self.0 { let _ = child.kill(); let _ = child.wait(); } } }
pub fn launch(root: &Path, installed: &Installed, resume_hosting: bool, timeout: Duration) -> anyhow::Result<Child> {
    install::validate(root, installed)?;
    ensure!(Some(installed.platform.as_str()) == super::Platform::current().name(), "Update is for a different platform");
    let job = install::job_directory(root)?; let path = job.0.join("ready.json");
    let mut random = [0; 32]; ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut random).map_err(|_| anyhow::anyhow!("OS randomness unavailable"))?;
    let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let mut command = Command::new(installed.executable());
    command.arg("--update-ready-file").arg(&path).arg("--update-nonce").arg(&nonce).env("YO3_DESKTOP_DATA", root)
        .env_remove("YO3_BUNDLE_DIR").env_remove("YO3_DESKTOP_BIN").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if resume_hosting { command.arg("--resume-hosting"); }
    #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
    let mut child = Candidate(Some(command.spawn().context("The new launcher could not start; the current version is still available")?));
    let deadline = Instant::now() + timeout;
    loop {
        let process = child.0.as_mut().unwrap(); ensure!(process.try_wait()?.is_none(), "The new launcher exited before it was ready; the current version remains active");
        if path.exists() {
            let meta = std::fs::symlink_metadata(&path)?;
            ensure!(meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= 4096, "Invalid update readiness reply");
            if let Ok(ack) = serde_json::from_slice::<Ack>(&std::fs::read(&path)?) {
                ensure!(ack.nonce == nonce && ack.pid == process.id() && super::version(&ack.version)? == super::version(&installed.tag)?, "The new launcher reported the wrong version or identity");
                install::promote(root, installed)?; return Ok(child.0.take().unwrap())
            }
        }
        ensure!(Instant::now() < deadline, "The new launcher did not become ready; the current version remains active");
        std::thread::sleep(Duration::from_millis(50));
    }
}
