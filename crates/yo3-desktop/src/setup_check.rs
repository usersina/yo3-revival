//! The setup wizard's checks (#28). Where the game might be installed, then one line per check: the supported game,
//! readable game data, Wine (Linux), the game's DirectX and Visual C++ runtimes, whether direct start can write to the
//! folder, and YO3 Revival's own files. Each line passes, warns or fails with one sentence saying how to fix it; the
//! technical text stays behind Details. A fingerprint of `yo3.exe` lets later starts skip the slow checks.
use std::io::Read;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::time::Duration;
use crate::{Backend, Settings};

/// A folder that looks like an installed game, with the Wine prefix it lives in (Linux).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate { pub game_dir: PathBuf, pub wine_prefix: Option<PathBuf> }

const KONAMI: [&str; 2] = ["Program Files (x86)/Konami/Yu-Gi-Oh! ONLINE 3", "Program Files/Konami/Yu-Gi-Oh! ONLINE 3"];

/// Installed copies found in the usual places: Program Files on every drive (Windows); on Linux the default Wine
/// prefix, other `~/.wine*` prefixes, and Lutris and Bottles prefixes.
pub fn candidates() -> Vec<Candidate> {
    let mut found = Vec::new();
    #[cfg(windows)] {
        for drive in b'C'..=b'Z' {
            let root = PathBuf::from(format!("{}:\\", drive as char));
            for place in KONAMI { push_game(&mut found, root.join(place.replace('/', "\\")), None) }
        }
    }
    #[cfg(not(windows))] {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        for prefix in linux_prefixes(&home) {
            for place in KONAMI { push_game(&mut found, prefix.join("drive_c").join(place), Some(prefix.clone())) }
        }
    }
    found
}

fn push_game(found: &mut Vec<Candidate>, dir: PathBuf, wine_prefix: Option<PathBuf>) {
    if dir.join("yo3.exe").is_file() && !found.iter().any(|c| c.game_dir == dir) { found.push(Candidate { game_dir: dir, wine_prefix }) }
}

/// Wine prefixes under `home`, the default first.
#[cfg(not(windows))]
pub fn linux_prefixes(home: &Path) -> Vec<PathBuf> {
    let mut prefixes = vec![home.join(".wine")];
    let children = |dir: PathBuf| -> Vec<PathBuf> {
        let mut list: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        list.sort();
        list
    };
    prefixes.extend(children(home.to_owned()).into_iter().filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(".wine") && n != ".wine")));
    for place in ["Games", ".local/share/lutris/prefixes", ".local/share/bottles/bottles", ".var/app/com.usebottles.bottles/data/bottles/bottles"] {
        prefixes.extend(children(home.join(place)));
    }
    prefixes.retain(|p| p.join("drive_c").is_dir());
    prefixes
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check { Game, Data, Wine, Runtimes, DirectStart, Ours }

impl Check {
    /// The line's title when it passes.
    pub fn title(self) -> &'static str {
        match self {
            Check::Game => "Yu-Gi-Oh! ONLINE 3, the supported version",
            Check::Data => "Game data can be read",
            Check::Wine => "Wine is installed",
            Check::Runtimes => "DirectX 9 and Visual C++ 2008 runtimes",
            Check::DirectStart => "Opening the game without the launcher",
            Check::Ours => "YO3 Revival's own files",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State { Pass, Warn, Fail, Skipped }

/// One line of the checklist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line { pub check: Check, pub state: State, pub sentence: String, pub details: Option<String> }

impl Line {
    fn pass(check: Check) -> Self { Self { check, state: State::Pass, sentence: String::new(), details: None } }
    fn new(check: Check, state: State, sentence: &str, details: Option<String>) -> Self { Self { check, state, sentence: sentence.into(), details } }
}

/// Whether the wizard can go on: the game, its data, Wine and the runtimes must pass. A warning only informs, and
/// YO3 Revival's own files are the main screen's business (its Repair state).
pub fn passes(lines: &[Line]) -> bool {
    lines.iter().all(|l| l.state != State::Fail || l.check == Check::Ours)
}

/// The checks that need only the folder: the game, its data and write access for direct start.
pub fn game_lines(dir: &Path) -> Vec<Line> {
    let exe = dir.join("yo3.exe");
    if !exe.is_file() {
        return vec![Line::new(Check::Game, State::Fail, "There's no yo3.exe here. Choose the folder Yu-Gi-Oh! ONLINE 3 is installed in.", None)];
    }
    let mut lines = Vec::new();
    match yo3_launcher::components::inspect_game(&exe) {
        Ok(_) => lines.push(Line::pass(Check::Game)),
        Err(error) => {
            lines.push(Line::new(Check::Game, State::Fail, "This isn't the supported version of Yu-Gi-Oh! ONLINE 3 (1.153). Update the game, or choose another copy.", Some(format!("{error:#}"))));
            return lines;
        }
    }
    let data = if dir.join("yo3.idx").is_file() { yo3_data::GameData::load(dir).map(|_| ()).map_err(anyhow::Error::from) } else { Err(anyhow::anyhow!("yo3.idx is missing")) };
    lines.push(match data {
        Ok(()) => Line::pass(Check::Data),
        Err(error) => Line::new(Check::Data, State::Fail, "Game data can't be read. Reinstall the game, or choose a complete copy.", Some(format!("{error:#}"))),
    });
    lines.push(write_access(dir));
    lines
}

/// Direct start puts a small loader beside `yo3.exe`: can we write there?
fn write_access(dir: &Path) -> Line {
    let probe = dir.join(".yo3-revival-write-check");
    let written = std::fs::OpenOptions::new().write(true).create_new(true).open(&probe);
    match written {
        Ok(_) => { let _ = std::fs::remove_file(&probe); Line::pass(Check::DirectStart) }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => { let _ = std::fs::remove_file(&probe); write_access(dir) }
        Err(error) if error.kind() == std::io::ErrorKind::ReadOnlyFilesystem => Line::new(Check::DirectStart, State::Warn,
            "This folder is read-only, so the game opens only from YO3 Revival.", Some(error.to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Line::new(Check::DirectStart, State::Warn,
            "This folder is protected. Opening the game without the launcher will ask for permission once.", Some(error.to_string())),
        Err(error) => Line::new(Check::DirectStart, State::Warn, "YO3 Revival can't write to this folder, so the game opens only from YO3 Revival.", Some(error.to_string())),
    }
}

/// Wine itself and the prefix (Linux only).
#[cfg(not(windows))]
pub fn wine_line(prefix: Option<&Path>, wine: &str) -> Line {
    if let Some(prefix) = prefix && !prefix.join("drive_c").is_dir() {
        return Line::new(Check::Wine, State::Fail, "This Wine prefix doesn't exist. Choose the prefix the game is installed in under More options.", Some(prefix.display().to_string()));
    }
    let mut cmd = std::process::Command::new(yo3_launcher::runtime::wine_tool_of(Some(wine.as_ref()), "wine"));
    cmd.arg("--version").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());
    match run_bounded(&mut cmd, Duration::from_secs(15)) {
        Ok(version) => Line { details: Some(version.trim().to_owned()), ..Line::pass(Check::Wine) },
        Err(error) => Line::new(Check::Wine, State::Fail, &install_wine_hint(), Some(format!("{error:#}"))),
    }
}

/// How to install Wine on this distribution.
#[cfg(not(windows))]
fn install_wine_hint() -> String {
    let release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let family = release.lines().filter_map(|l| l.strip_prefix("ID=").or_else(|| l.strip_prefix("ID_LIKE="))).collect::<Vec<_>>().join(" ");
    let command = if family.contains("arch") { "sudo pacman -S wine" } else if family.contains("debian") || family.contains("ubuntu") { "sudo apt install wine" }
        else if family.contains("fedora") || family.contains("rhel") { "sudo dnf install wine" } else if family.contains("suse") { "sudo zypper install wine" }
        else { "your distribution's package manager" };
    format!("Wine isn't installed. Install it with {command}, then Retry.")
}

#[cfg(not(windows))]
fn run_bounded(cmd: &mut std::process::Command, limit: Duration) -> anyhow::Result<String> {
    let mut child = cmd.spawn()?;
    let started = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let mut out = String::new();
            if let Some(mut stdout) = child.stdout.take() { stdout.read_to_string(&mut out)?; }
            anyhow::ensure!(status.success(), "exited with {status}");
            return Ok(out);
        }
        if started.elapsed() > limit { let _ = child.kill(); let _ = child.wait(); anyhow::bail!("no answer within {} seconds", limit.as_secs()) }
        std::thread::sleep(Duration::from_millis(50));
    }
}

impl Backend {
    /// Every check for `settings`, in the wizard's order. Blocking: run it off the window's thread.
    pub fn setup_checklist(&self, settings: &Settings) -> Vec<Line> {
        let dir = Path::new(&settings.game_dir);
        let mut lines = game_lines(dir);
        let game_ok = lines.iter().all(|l| l.state != State::Fail);
        #[cfg(not(windows))]
        let wine_ok = {
            let line = wine_line((!settings.wine_prefix.is_empty()).then(|| Path::new(&settings.wine_prefix)), &settings.wine_binary);
            let ok = line.state == State::Pass;
            lines.insert(lines.len().min(2), line);
            ok
        };
        #[cfg(windows)]
        let wine_ok = true;
        // Our own files are checked against the game's yo3.exe, so a wrong folder must not read as "needs repair".
        if !game_ok { return lines }
        let ours = self.check_components(settings).and_then(|()| self.check_server_resources());
        lines.push(match (wine_ok, ours.is_ok()) {
            (true, true) => match self.check_game(settings) {
                Ok(()) => Line::pass(Check::Runtimes),
                Err(error) => {
                    let text = format!("{error:#}");
                    let sentence = text.lines().next().unwrap_or("The game's runtime check failed.").to_owned();
                    Line { check: Check::Runtimes, state: State::Fail, sentence, details: Some(text) }
                }
            },
            (true, false) => Line::new(Check::Runtimes, State::Skipped, "Checked once YO3 Revival is repaired.", None),
            (false, _) => Line::new(Check::Runtimes, State::Skipped, "Checked once Wine works.", None),
        });
        lines.push(match ours {
            Ok(()) => Line::pass(Check::Ours),
            Err(error) => Line::new(Check::Ours, State::Fail, "YO3 Revival needs repair.", Some(format!("{error:#}"))),
        });
        lines
    }

    /// A quick answer at startup when the game is unchanged since the last full check: only YO3 Revival's own files
    /// are looked at again. `None` means the game changed (or was never checked) and needs the full check.
    pub fn quick_readiness(&self, settings: &Settings) -> Option<crate::Readiness> {
        let current = fingerprint(Path::new(&settings.game_dir)).ok()?;
        if settings.game_fingerprint.is_empty() || current != settings.game_fingerprint { return None }
        Some(match self.check_components(settings) {
            Ok(()) => crate::Readiness::Ready,
            Err(error) => crate::Readiness::ComponentProblem(format!("{error:#}")),
        })
    }
}

/// `yo3.exe`'s size, modification time and SHA-256: cheap to read, and it changes when the game is updated,
/// replaced or moved.
pub fn fingerprint(dir: &Path) -> anyhow::Result<String> {
    let exe = dir.join("yo3.exe");
    let meta = std::fs::metadata(&exe)?;
    anyhow::ensure!(meta.len() <= 64 * 1024 * 1024, "yo3.exe is unexpectedly large");
    let modified = meta.modified()?.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default();
    let mut digest = ring::digest::Context::new(&ring::digest::SHA256);
    let mut file = std::fs::File::open(&exe)?;
    let mut buffer = [0; 16384];
    loop { let read = file.read(&mut buffer)?; if read == 0 { break } digest.update(&buffer[..read]); }
    let hash: String = digest.finish().as_ref().iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("{}:{modified}:{hash}", meta.len()))
}
