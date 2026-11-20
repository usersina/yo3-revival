//! What the window knows about an update (#32): when to check again, the download in the background, and what it shows
//! (the footer's dot, the ribbon above PLAY, What's new). Toolkit-free, so each step is tested without a window.
use std::time::{Duration, Instant};
use super::{Decision, Release, install::Installed};

/// Checked at start, then this often while the launcher is open.
pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// Where the newest offered version is.
#[derive(Clone, Debug)]
pub enum Stage {
    /// Nothing to download (none offered, or it couldn't start before: see `failed_start`).
    Idle,
    /// Downloading and checking it in the background.
    Downloading(Release),
    /// Checked and unpacked beside this version: it installs when the launcher restarts.
    Ready(Release, Installed),
    /// The download or its check failed: never installed. Tried again at the next check.
    Failed(Release, String),
}

#[derive(Debug)]
pub struct Updater {
    /// The last answer from the release check (an offline check keeps the one before).
    pub latest: Option<Decision>,
    pub stage: Stage,
    checking: bool,
    last_check: Option<Instant>,
    /// A version that was installed but didn't start: it stays offered, but isn't downloaded again by itself.
    failed_start: Option<String>,
}

/// What the Update remedy of a server that needs a newer launcher does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remedy { WhatsNew, Releases }

impl Updater {
    pub fn new(failed_start: Option<String>) -> Self {
        Self { latest: None, stage: Stage::Idle, checking: false, last_check: None, failed_start }
    }

    /// Whether to check now: never checked, or `CHECK_EVERY` since the last, and nothing in flight.
    pub fn due(&self, now: Instant) -> bool {
        !self.checking && !matches!(self.stage, Stage::Downloading(_))
            && self.last_check.is_none_or(|last| now.saturating_duration_since(last) >= CHECK_EVERY)
    }

    /// A check starts.
    pub fn checking(&mut self, now: Instant) { self.checking = true; self.last_check = Some(now) }

    /// The check's answer. Returns the release to download, when there is a new one to fetch.
    pub fn checked(&mut self, result: Result<Decision, String>) -> Option<Release> {
        self.checking = false;
        let Ok(decision) = result else { return None };
        let offered = match &decision { Decision::Available(release) => Some(release.clone()), _ => None };
        self.latest = Some(decision);
        let release = offered?;
        if self.failed_start.as_deref() == Some(release.tag.as_str()) { return None }
        match &self.stage {
            Stage::Downloading(r) | Stage::Ready(r, _) if r.tag == release.tag => None,
            _ => { self.stage = Stage::Downloading(release.clone()); Some(release) }
        }
    }

    /// The background download of `tag` finished.
    pub fn prepared(&mut self, tag: &str, result: Result<Installed, String>) {
        let Stage::Downloading(release) = &self.stage else { return };
        if release.tag != tag { return }
        let release = release.clone();
        self.stage = match result { Ok(installed) => Stage::Ready(release, installed), Err(error) => Stage::Failed(release, error) };
    }

    /// The ready version didn't start (a restart into it failed): this version stays, and it isn't downloaded again.
    pub fn start_failed(&mut self, tag: &str) {
        self.failed_start = Some(tag.to_owned());
        if matches!(&self.stage, Stage::Ready(r, _) if r.tag == tag) { self.stage = Stage::Idle }
    }

    /// The version offered (without its `v`), for the footer's dot.
    pub fn offered(&self) -> Option<&str> { self.release().map(|r| r.tag.trim_start_matches('v')) }

    /// The release the window talks about: the one downloading or ready, else the one offered.
    pub fn release(&self) -> Option<&Release> {
        match &self.stage {
            Stage::Downloading(r) | Stage::Ready(r, _) | Stage::Failed(r, _) => Some(r),
            Stage::Idle => match &self.latest { Some(Decision::Available(r)) => Some(r), _ => None },
        }
    }

    /// Ready to install on restart.
    pub fn ready(&self) -> Option<&Installed> { match &self.stage { Stage::Ready(_, installed) => Some(installed), _ => None } }

    /// One line for What's new about where the update is.
    pub fn status(&self, playing: bool) -> String {
        match &self.stage {
            Stage::Downloading(_) => "Downloading in the background. You can keep playing.".into(),
            Stage::Ready(..) if playing => "Downloaded and checked. It installs when you close the game and restart YO3 Revival.".into(),
            Stage::Ready(..) => "Downloaded and checked. Restart YO3 Revival to install it.".into(),
            Stage::Failed(..) => "The download didn't pass its check, so it wasn't installed. It is tried again later.".into(),
            Stage::Idle if self.release().is_some_and(|r| self.failed_start.as_deref() == Some(r.tag.as_str())) =>
                "This version didn't start last time, so YO3 Revival kept your current version. Download it from the release page.".into(),
            Stage::Idle => "Download it from the release page.".into(),
        }
    }

    /// What a server's Update button does: What's new when we know the update, else the releases page.
    pub fn remedy(&self) -> Remedy { if self.release().is_some() { Remedy::WhatsNew } else { Remedy::Releases } }
}

/// The bar of an update being downloaded: how far (0 to 1, none while it is being checked) and its words, from the bytes received.
pub fn download_line(received: u64, total: u64, verifying: bool) -> (Option<f32>, String) {
    if verifying { return (None, "Checking the download…".into()) }
    if received == 0 || total == 0 { return (None, "Starting the download…".into()) }
    let mb = |bytes: u64| bytes as f64 / 1_048_576.0;
    (Some((received as f32 / total as f32).clamp(0.0, 1.0)), format!("Downloading: {:.1} of {:.1} MB", mb(received.min(total)), mb(total)))
}

/// The ribbon above PLAY when an update is ready: its sentence, and whether Restart to install is offered.
pub fn ribbon(version: &str, playing: bool) -> (String, bool) {
    if playing { (format!("Update {version} is ready · Installs after you close the game"), false) }
    else { (format!("Update {version} is ready ·"), true) }
}

/// The version's changelog section from its release notes: the notes up to the bundle's Downloads and Checksums.
pub fn whats_new(notes: &str) -> String {
    let end = ["\n## Downloads", "\n## Checksums"].iter().filter_map(|h| notes.find(h)).min().unwrap_or(notes.len());
    notes[..end].trim().to_owned()
}

/// One block of What's new: a paragraph or a bullet, as (text, bold) runs (`**…**`); links keep their words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block { pub bullet: bool, pub runs: Vec<(String, bool)> }

/// The changelog's Markdown as plain blocks: paragraphs and `- ` bullets (wrapped lines joined), bold kept, links' words.
pub fn blocks(markdown: &str) -> Vec<Block> {
    let mut out: Vec<(bool, String)> = Vec::new();
    let mut open = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { open = false; continue }
        if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) { out.push((true, rest.to_owned())); open = true; continue }
        let heading = trimmed.trim_start_matches('#');
        if heading.len() != trimmed.len() && heading.starts_with(' ') { out.push((false, format!("**{}**", heading.trim()))); open = false; continue }
        match out.last_mut() { Some((_, text)) if open => { text.push(' '); text.push_str(trimmed) } _ => { out.push((false, trimmed.to_owned())); open = true } }
    }
    out.into_iter().map(|(bullet, text)| Block { bullet, runs: runs(&unlink(&text)) }).collect()
}

/// `[words](url)` → `words`; inline code loses its backticks.
fn unlink(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        let Some(close) = rest[start..].find("](").map(|i| start + i) else { break };
        let Some(end) = rest[close..].find(')').map(|i| close + i) else { break };
        out.push_str(&rest[..start]);
        out.push_str(&rest[start + 1..close]);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out.replace('`', "")
}

fn runs(text: &str) -> Vec<(String, bool)> {
    text.split("**").enumerate().filter(|(_, part)| !part.is_empty()).map(|(i, part)| (part.to_owned(), i % 2 == 1)).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_download_says_how_far_it_is_and_waits_without_a_length_before_and_after() {
        assert_eq!(super::download_line(0, 50_000_000, false), (None, "Starting the download…".to_owned()));
        assert_eq!(super::download_line(26_214_400, 52_428_800, false), (Some(0.5), "Downloading: 25.0 of 50.0 MB".to_owned()));
        assert_eq!(super::download_line(52_428_800, 52_428_800, true), (None, "Checking the download…".to_owned()));
        assert_eq!(super::download_line(60_000_000, 52_428_800, false).0, Some(1.0), "never past the end");
        for words in ["Starting the download…", "Checking the download…", "Downloading: 25.0 of 50.0 MB"] { assert_eq!(crate::words::technical(words), None) }
    }

    use super::*;
    use super::super::Asset;

    fn release(tag: &str) -> Release {
        Release { tag: tag.into(), url: format!("https://github.com/usersina/yo3-revival/releases/tag/{tag}"), notes: "notes".into(),
            asset: Asset { name: String::new(), url: String::new(), bytes: 1, sha256: String::new() } }
    }
    fn installed(tag: &str) -> Installed {
        Installed { tag: tag.into(), directory: Default::default(), platform: "linux-x86_64".into(), manifest_sha256: String::new(),
            desktop_sha256: String::new(), archive_sha256: String::new() }
    }

    #[test]
    fn checks_at_start_then_every_six_hours_and_never_twice_at_once() {
        let mut updater = Updater::new(None);
        let start = Instant::now();
        assert!(updater.due(start), "the first check is at start");
        updater.checking(start);
        assert!(!updater.due(start + CHECK_EVERY), "one check at a time");
        assert!(updater.checked(Err("offline".into())).is_none());
        assert!(!updater.due(start + CHECK_EVERY - Duration::from_secs(1)));
        assert!(updater.due(start + CHECK_EVERY), "again six hours later");
        updater.checking(start + CHECK_EVERY);
        assert!(updater.checked(Ok(Decision::Available(release("v0.2.1")))).is_some());
        assert!(!updater.due(start + CHECK_EVERY * 3), "not while the download runs");
    }

    #[test]
    fn an_offered_version_downloads_once_and_is_ready_only_when_checked() {
        let mut updater = Updater::new(None);
        assert_eq!(updater.checked(Ok(Decision::Available(release("v0.2.1")))).map(|r| r.tag), Some("v0.2.1".into()));
        assert_eq!(updater.offered(), Some("0.2.1"), "the dot shows while it downloads");
        assert!(updater.ready().is_none(), "no ribbon before the download is checked");
        assert!(updater.checked(Ok(Decision::Available(release("v0.2.1")))).is_none(), "the same version isn't fetched twice");
        updater.prepared("v0.2.0", Ok(installed("v0.2.0")));
        assert!(updater.ready().is_none(), "a stale download's answer is ignored");
        updater.prepared("v0.2.1", Ok(installed("v0.2.1")));
        assert_eq!(updater.ready().map(|i| i.tag.as_str()), Some("v0.2.1"));
        assert!(updater.checked(Ok(Decision::Available(release("v0.2.1")))).is_none(), "ready stays ready");
        assert_eq!(updater.checked(Ok(Decision::Available(release("v0.2.2")))).map(|r| r.tag), Some("v0.2.2".into()), "a newer one replaces it");
        assert!(updater.ready().is_none());
    }

    #[test]
    fn a_failed_check_of_the_download_is_reported_never_installed_and_tried_again() {
        let mut updater = Updater::new(None);
        updater.checked(Ok(Decision::Available(release("v0.2.1"))));
        updater.prepared("v0.2.1", Err("Update download checksum or length does not match GitHub".into()));
        assert!(updater.ready().is_none());
        assert!(updater.status(false).contains("wasn't installed"));
        assert!(updater.checked(Ok(Decision::Available(release("v0.2.1")))).is_some(), "the next check tries again");
    }

    #[test]
    fn a_version_that_failed_to_start_stays_offered_but_isnt_downloaded_again() {
        let mut updater = Updater::new(Some("v0.2.1".into()));
        assert!(updater.checked(Ok(Decision::Available(release("v0.2.1")))).is_none());
        assert_eq!(updater.offered(), Some("0.2.1"));
        assert!(updater.status(false).contains("didn't start"));
        assert!(updater.checked(Ok(Decision::Available(release("v0.2.2")))).is_some(), "a newer release is fetched");
    }

    #[test]
    fn a_restart_into_the_update_that_fails_keeps_this_version_and_drops_the_ribbon() {
        let mut updater = Updater::new(None);
        updater.checked(Ok(Decision::Available(release("v0.2.1"))));
        updater.prepared("v0.2.1", Ok(installed("v0.2.1")));
        updater.start_failed("v0.2.1");
        assert!(updater.ready().is_none(), "no Restart to install for a version that didn't start");
        assert_eq!(updater.offered(), Some("0.2.1"));
        assert!(updater.checked(Ok(Decision::Available(release("v0.2.1")))).is_none());
    }

    #[test]
    fn an_offline_check_keeps_what_was_known() {
        let mut updater = Updater::new(None);
        updater.checked(Ok(Decision::Available(release("v0.2.1"))));
        updater.checked(Err("offline".into()));
        assert_eq!(updater.offered(), Some("0.2.1"));
        let mut current = Updater::new(None);
        current.checked(Ok(Decision::Current { latest: "v0.2.0".into() }));
        assert_eq!(current.offered(), None, "no dot when up to date");
    }

    #[test]
    fn the_ribbon_offers_restart_only_when_the_game_is_closed() {
        assert_eq!(ribbon("0.2.1", false), ("Update 0.2.1 is ready ·".into(), true));
        let (line, restart) = ribbon("0.2.1", true);
        assert!(!restart && line.contains("after you close the game"));
        let mut updater = Updater::new(None);
        updater.checked(Ok(Decision::Available(release("v0.2.1"))));
        updater.prepared("v0.2.1", Ok(installed("v0.2.1")));
        assert!(updater.status(true).contains("close the game"));
    }

    #[test]
    fn a_server_that_needs_a_newer_launcher_opens_whats_new_when_the_update_is_known() {
        let mut updater = Updater::new(None);
        assert_eq!(updater.remedy(), Remedy::Releases);
        updater.checked(Ok(Decision::Available(release("v0.2.1"))));
        assert_eq!(updater.remedy(), Remedy::WhatsNew);
    }

    #[test]
    fn whats_new_is_the_changelog_section_of_the_release_notes() {
        let notes = "Token fixes.\n\n- **Choosing a token's position** no longer\n  freezes [the duel](https://x).\n- `Sweet Corn` appears.\n\n## Downloads\n\n| a | b |\n\n## Checksums (SHA-256)\n\nabc";
        let section = whats_new(notes);
        assert!(section.ends_with("appears.") && !section.contains("Downloads") && !section.contains("abc"));
        assert_eq!(blocks(&section), vec![
            Block { bullet: false, runs: vec![("Token fixes.".into(), false)] },
            Block { bullet: true, runs: vec![("Choosing a token's position".into(), true), (" no longer freezes the duel.".into(), false)] },
            Block { bullet: true, runs: vec![("Sweet Corn appears.".into(), false)] },
        ]);
        assert_eq!(whats_new("Only the section."), "Only the section.");
    }
}
