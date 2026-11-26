//! The anime card pictures (#60) on the player's path: Get pictures (downloaded from ProjectIgnis' picture server on the
//! player's click, converted here), Add from a file and Save to a file. The rules live in
//! `yo3_launcher::art` (`get`, `pack`); this is the HTTPS asker, the thread the download runs on and the sentences.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use yo3_launcher::art::get::{self, Fetched, Fetcher, Outcome};
use yo3_launcher::art::pack;

/// ProjectIgnis' picture server: `{passcode}.jpg` under it, and a Field Spell's duel field `field/{passcode}.png`.
pub const SERVER: &str = "https://pics.projectignis.org:2096/pics/";
/// The duel field pictures' folder on the server (a URL piece, never shown).
const FIELDS: &str = concat!("field", "/");
/// Said to the server: only that this is a program, nothing about the player.
const AGENT: &str = "YO3-Revival-Pictures";

/// Asks the picture server over HTTPS, one request at a time, with timeouts. Not made on an async thread: it runs its own
/// small runtime.
pub struct HttpFetcher { base: String, client: reqwest::Client, runtime: tokio::runtime::Runtime }

impl HttpFetcher {
    pub fn new(base: &str) -> anyhow::Result<Self> {
        let client = crate::github_http::builder().https_only(base.starts_with("https://")).user_agent(AGENT).connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30)).redirect(reqwest::redirect::Policy::limited(3)).build()?;
        Ok(Self { base: base.to_owned(), client, runtime: tokio::runtime::Builder::new_current_thread().enable_all().build()? })
    }
}

impl HttpFetcher {
    /// `name` under the server's folder: the picture's bytes, `Missing` on a 404.
    fn get(&mut self, name: &str) -> Result<Fetched, String> {
        let url = format!("{}{name}", self.base);
        self.runtime.block_on(async {
            let mut response = self.client.get(&url).send().await.map_err(|e| e.without_url().to_string())?;
            if response.status() == reqwest::StatusCode::NOT_FOUND { return Ok(Fetched::Missing) }
            if !response.status().is_success() { return Err(format!("the server answered {}", response.status().as_u16())) }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|e| e.without_url().to_string())? {
                bytes.extend_from_slice(&chunk);
                if bytes.len() > get::MAX_BYTES { return Err("the picture is too large".into()) }
            }
            Ok(Fetched::Jpeg(bytes))
        })
    }
}

impl Fetcher for HttpFetcher {
    fn fetch(&mut self, passcode: u32) -> Result<Fetched, String> { self.get(&format!("{passcode}.jpg")) }
    /// EDOPro's duel field picture: `field/{passcode}.png`.
    fn fetch_field(&mut self, passcode: u32) -> Result<Fetched, String> { self.get(&format!("{FIELDS}{passcode}.png")) }
}

/// What the download thread reports.
#[derive(Clone, Debug)]
pub enum Event { Progress(u32, u32), Done(Result<Outcome, String>) }

/// Starts the download on a thread of its own (the player clicked): `events` gets the progress, then the end. Setting `cancel`
/// stops it at the next card.
pub fn start(dir: PathBuf, base: String, cancel: Arc<AtomicBool>, events: tokio::sync::mpsc::UnboundedSender<Event>) {
    std::thread::spawn(move || {
        let result = HttpFetcher::new(&base).map_err(|e| format!("{e:#}")).and_then(|mut fetcher| {
            let mut progress = |done, total| { let _ = events.send(Event::Progress(done, total)); };
            get::download(&dir, &get::list(), &get::fields(), &mut fetcher, get::PACE, &cancel, &mut progress).map_err(|e| e.to_string())
        });
        let _ = events.send(Event::Done(result));
    });
}

/// (pictures in the folder, cards that have one to get) for the folder `dir`.
pub fn count(dir: &Path) -> (usize, usize) {
    let all = get::list();
    (get::have(dir, &all), all.len())
}

pub fn stop(cancel: &AtomicBool) { cancel.store(true, Ordering::SeqCst) }

/// The sentence after a download, and whether it went well.
pub fn download_sentence(out: &Outcome) -> (bool, String) {
    let got = out.saved.len() + out.fields.len();
    let pictures = |n: usize| if n == 1 { "1 picture".to_owned() } else { format!("{n} pictures") };
    if out.unreachable.is_some() {
        return (false, format!("We couldn't reach the picture server{}. Check your internet connection and try again, or use Add from a file.",
            if got > 0 { format!(", so it stopped after {}", pictures(got)) } else { String::new() }))
    }
    if out.cancelled { return (true, format!("Stopped after {}. Get the rest carries on from there.", pictures(got))) }
    if out.failed.is_empty() { return (true, if got == 0 { "You already have every picture.".into() } else { format!("Got {}.", pictures(got)) }) }
    let missing = out.failed.len();
    (false, format!("Got {}. {} couldn't be fetched: the game keeps its own artwork for {}. Get the rest tries them again, or use Add from a file.",
        pictures(got), if missing == 1 { "1 card".to_owned() } else { format!("{missing} cards") }, if missing == 1 { "it" } else { "them" }))
}

/// The sentence after adding pictures from a file.
pub fn import_sentence(done: &pack::Imported) -> (bool, String) {
    let added = done.faces + done.minis;
    let mut text = if added == 0 { "No new pictures were added.".to_owned() } else { format!("Added {} picture{}.", done.faces.max(done.minis), if done.faces.max(done.minis) == 1 { "" } else { "s" }) };
    if done.kept > 0 { text.push_str(&format!(" {} of yours {} kept.", done.kept, if done.kept == 1 { "was" } else { "were" })) }
    if !done.refused.is_empty() { text.push_str(&format!(" {} file{} skipped: not a card picture of the right size.", done.refused.len(), if done.refused.len() == 1 { " was" } else { "s were" })) }
    (added > 0 || done.refused.is_empty(), text)
}

pub fn export_sentence(done: &pack::Exported) -> String {
    format!("Saved {} picture{}.", done.faces, if done.faces == 1 { "" } else { "s" })
}

/// The download size said before the click.
pub const SIZE_NOTE: &str = get::DOWNLOAD_SIZE;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words::technical;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A local server: 404 for passcode 1, 503 for 3, the bytes `\xff\xd8\xffxx` for the rest; keeps each request's text.
    fn serve() -> (String, Arc<std::sync::Mutex<Vec<String>>>) {
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}/pics/", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let log = seen.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            rt.block_on(async {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                loop {
                    let Ok((mut socket, _)) = listener.accept().await else { continue };
                    let log = log.clone();
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 4096];
                        let n = socket.read(&mut buf).await.unwrap_or(0);
                        let request = String::from_utf8_lossy(&buf[..n]).into_owned();
                        let (status, body): (&str, &[u8]) = if request.starts_with("GET /pics/1.jpg") { ("404 Not Found", b"") } else if request.starts_with("GET /pics/3.jpg") { ("503 Service Unavailable", b"") } else { ("200 OK", b"\xff\xd8\xffxx") };
                        log.lock().unwrap().push(request);
                        let _ = socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await;
                        let _ = socket.write_all(body).await;
                    });
                }
            });
        });
        (base, seen)
    }

    #[test]
    fn the_fetcher_tells_a_picture_from_a_404_and_an_error_and_says_nothing_about_the_player() {
        let (base, seen) = serve();
        let mut fetcher = HttpFetcher::new(&base).unwrap();
        assert_eq!(fetcher.fetch(2).unwrap(), Fetched::Jpeg(b"\xff\xd8\xffxx".to_vec()));
        assert_eq!(fetcher.fetch(1).unwrap(), Fetched::Missing);
        assert!(fetcher.fetch(3).unwrap_err().contains("503"));
        let requests = seen.lock().unwrap().join("\n").to_lowercase();
        assert!(requests.contains("user-agent: yo3-revival-pictures"), "{requests}");
        for private in ["cookie", "authorization", "referer", "@"] { assert!(!requests.contains(private), "{private}: {requests}") }
        let mut down = HttpFetcher::new("http://127.0.0.1:1/pics/").unwrap();
        assert!(down.fetch(2).is_err(), "a server that can't be reached is an error, not a missing picture");
    }

    #[test]
    fn a_download_thread_reports_progress_and_the_end_and_keeps_to_the_folder() {
        let (base, _) = serve();
        let dir = std::env::temp_dir().join(format!("yo3-pictures-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let cancel = Arc::new(AtomicBool::new(true));
        start(dir.clone(), base, cancel, tx);
        let mut last = None;
        while let Some(event) = rx.blocking_recv() { if let Event::Done(result) = event { last = Some(result); break } }
        let out = last.unwrap().unwrap();
        assert!(out.cancelled && out.saved.is_empty(), "cancelled before the first card: nothing asked");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// By hand only, against the real server and for three cards (never the whole set): `YO3_LIVE_PICTURES=/scratch/dir cargo test
    /// -p yo3-desktop --lib live_three_cards -- --ignored`.
    #[test]
    #[ignore]
    fn live_three_cards() {
        let dir = PathBuf::from(std::env::var("YO3_LIVE_PICTURES").expect("a scratch folder"));
        let entries: Vec<_> = get::list().into_iter().filter(|e| [4021, 8035, 4998].contains(&e.cid)).collect();
        let mut fetcher = HttpFetcher::new(SERVER).unwrap();
        let out = get::download(&dir, &entries, &[], &mut fetcher, get::PACE, &AtomicBool::new(false), &mut |d, t| eprintln!("{d}/{t}")).unwrap();
        eprintln!("{out:?}");
        assert_eq!(out.saved.len(), 3, "{out:?}");
    }

    #[test]
    fn every_sentence_reads_plainly_and_points_to_the_file_when_the_server_does_not_answer() {
        let ok = Outcome { saved: vec![1, 2], ..Outcome::default() };
        let part = Outcome { saved: vec![1], failed: vec![(2, "x".into()), (3, "y".into())], ..Outcome::default() };
        let down = Outcome { unreachable: Some("connection refused".into()), failed: vec![(1, "z".into())], ..Outcome::default() };
        let stopped = Outcome { cancelled: true, saved: vec![5], ..Outcome::default() };
        let sentences = [download_sentence(&ok), download_sentence(&part), download_sentence(&down), download_sentence(&stopped), download_sentence(&Outcome::default()),
            import_sentence(&pack::Imported { faces: 3, minis: 1, kept: 2, refused: vec![("a".into(), "b".into())], ignored: 4 }), import_sentence(&pack::Imported::default()),
            (true, export_sentence(&pack::Exported { faces: 1, minis: 1 }))];
        for (_, text) in &sentences { assert_eq!(technical(text), None, "{text}") }
        assert!(down_text(&sentences[2]).contains("Add from a file") && !sentences[2].0);
        assert!(down_text(&sentences[1]).contains("game keeps its own artwork"));
        assert_eq!(sentences[0], (true, "Got 2 pictures.".to_owned()));
    }
    fn down_text(s: &(bool, String)) -> &str { &s.1 }
}
