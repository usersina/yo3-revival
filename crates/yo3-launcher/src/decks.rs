//! The player's local deck files follow the card profile (issue #60).
//!
//! The game keeps each player's decks as `players/<name>/decks/*.y3d` in its Documents folder, and its deck editor files a card
//! in the Main or Extra list by the card's kind. When a card profile makes a card another kind (Flame Swordsman is a Fusion
//! Monster in the game and a Normal Monster as the anime's), a deck saved under the other profile holds it in the wrong list.
//! Before the game is released, the launcher moves the cards the profile covers to the list the profile puts them in, with the
//! rule the server uses for its own decks (`yo3_proto::placement::place`): cards and quantities stay, nothing else changes, a
//! deck is never deleted. Each file is copied once to `<name>.y3d.pre-profile` before its first change, and the profile it
//! was adapted to is recorded in `players/<name>/revival-profile.txt`, so a start with the same profile does nothing.
//!
//! The folder is the game's own for this server: the original `Yu-Gi-Oh! ONLINE 3` folder for the first server played on,
//! `YO3 Revival/servers/<id>/Yu-Gi-Oh! ONLINE 3` beside it for every other (tools/mod/documents.h; [`folder_for`]).
//! The profile is the file the game itself will fetch (`GET /web/card-profile`), taken from the launcher's own relay.
use std::collections::{BTreeMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use yo3_proto::{Message, Value, decode_file, frame};

/// The suffix of the copy kept of a deck before its first change.
pub const BACKUP: &str = ".pre-profile";
/// The marker file in each player's folder.
pub const MARKER: &str = "revival-profile.txt";

/// The profile as far as deck placement is concerned: its hash and, for each card it covers, where it belongs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wanted {
    /// The profile's hash ("official" for none).
    pub hash: String,
    /// Each covered CARD_Prop row: whether the profile puts it in the Extra Deck.
    pub rows: BTreeMap<u16, bool>,
}

impl Wanted {
    /// The client profile file (`yo3-card-profile 1`, profile.rs): each `card` line's row and its `b` word, whose kind
    /// (bits 3-7) says the list: Fusion and Synchro Monsters (2, 3, 17-19) are Extra Deck cards. An empty or unreadable
    /// body is the game's own profile, which covers nothing.
    pub fn parse(file: &str) -> Wanted {
        let mut out = Wanted { hash: "official".into(), rows: BTreeMap::new() };
        let mut lines = file.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#'));
        if lines.next() != Some("yo3-card-profile 1") { return out }
        for line in lines {
            let word: Vec<&str> = line.split_whitespace().collect();
            match word.as_slice() {
                ["hash", hash] => out.hash = (*hash).to_string(),
                ["card", row, _cid, _a, b, ..] => {
                    if let (Ok(row), Ok(b)) = (row.parse::<u16>(), u32::from_str_radix(b, 16)) {
                        out.rows.insert(row, matches!(b >> 3 & 0x1F, 2 | 3 | 17..=19));
                    }
                }
                _ => {}
            }
        }
        if out.rows.is_empty() { out.hash = "official".into() }
        out
    }
}

/// The file beside a game's ready file that exists while the server pins another card profile than the game loaded: the
/// Desktop offers to restart the game then.
pub const STALE: &str = ".profile-stale";

/// The ready file's stale marker.
pub fn stale_file(ready: &Path) -> PathBuf {
    PathBuf::from(format!("{}{STALE}", ready.display()))
}

impl Wanted {
    /// The hash discovery announces for this profile (`ServerInfo::card_profile`): empty for the game's own.
    pub fn announced(&self) -> String {
        if self.hash == "official" { String::new() } else { self.hash.clone() }
    }
}

/// Notes whether the game still runs the profile the server pins: the marker file exists exactly when `announced` (what
/// discovery says now) is not what the game loaded. Returns whether it is stale.
pub fn note_staleness(ready: &Path, loaded: &str, announced: &str) -> bool {
    let (file, stale) = (stale_file(ready), loaded != announced);
    if stale { let _ = std::fs::write(&file, announced); } else { let _ = std::fs::remove_file(&file); }
    stale
}

/// What a run did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Players whose decks were looked at.
    pub players: usize,
    /// Deck files written.
    pub adapted: usize,
    /// Cards moved between lists, over all decks.
    pub moved: usize,
    /// Files left alone, with why (not a deck the game wrote, or not writable).
    pub skipped: Vec<(PathBuf, String)>,
}

/// Where the cards the profile moved were before it did (the game's own list): `true` the Extra Deck.
type Moved = BTreeMap<u16, bool>;

fn read_marker(path: &Path) -> Option<(String, Moved)> {
    let text = std::fs::read_to_string(path).ok()?;
    let (mut hash, mut moved) = (None, Moved::new());
    for line in text.lines() {
        match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["hash", h] => hash = Some((*h).to_string()),
            ["moved", row, list @ ("main" | "extra")] => { moved.insert(row.parse().ok()?, *list == "extra"); }
            _ => {}
        }
    }
    Some((hash?, moved))
}

fn write_marker(path: &Path, hash: &str, moved: &Moved) -> io::Result<()> {
    let mut text = format!("hash {hash}\n");
    for (row, extra) in moved { text += &format!("moved {row} {}\n", if *extra { "extra" } else { "main" }) }
    write_atomic(path, text.as_bytes())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = path.with_extension("tmp-profile");
    std::fs::write(&temp, bytes)?;
    std::fs::rename(&temp, path).inspect_err(|_| { let _ = std::fs::remove_file(&temp); })
}

fn rows(message: &Message, name: &str) -> Option<Vec<u16>> {
    match message.get(name)? {
        Value::Bin(bytes) if bytes.len() % 2 == 0 => Some(bytes.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect()),
        _ => None,
    }
}

fn set_rows(message: &mut Message, name: &str, rows: &[u16]) {
    if let Some(field) = message.fields.iter_mut().find(|(n, _)| n == name) { field.1 = Value::u16_be_array(rows) }
}

/// One deck's lists placed as `wanted` says, remembering in `moved` where each card that moved was before.
fn place(main: &[u16], extra: &[u16], wanted: &Wanted, moved: &mut Moved) -> (Vec<u16>, Vec<u16>) {
    let before = moved.clone();
    let want = |row: u16| wanted.rows.get(&row).copied().or_else(|| before.get(&row).copied());
    let (new_main, new_extra) = yo3_proto::placement::place(main, extra, want);
    for &row in main { if wanted.rows.get(&row) == Some(&true) { moved.entry(row).or_insert(false); } }
    for &row in extra { if wanted.rows.get(&row) == Some(&false) { moved.entry(row).or_insert(true); } }
    (new_main, new_extra)
}

/// The decks of every player in `folder` (the game's `Yu-Gi-Oh! ONLINE 3` folder), placed as `wanted` says. A player already
/// adapted to this profile is skipped; a file that is not a readable deck (or cannot be written) is left untouched and named
/// in the report.
pub fn adapt(folder: &Path, wanted: &Wanted) -> io::Result<Report> {
    let mut report = Report::default();
    let players = folder.join("players");
    let Ok(entries) = std::fs::read_dir(&players) else { return Ok(report) };
    let mut names: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    names.sort();
    for player in names {
        let marker = player.join(MARKER);
        let previous = read_marker(&marker);
        if previous.as_ref().is_some_and(|(hash, _)| *hash == wanted.hash) { continue }
        // A player with no marker was last at the game's own profile, unless nothing is covered: then there is nothing to undo.
        let mut moved = previous.map(|(_, moved)| moved).unwrap_or_default();
        // A card the new profile no longer covers goes back where the game has it; those leave `moved` after this run.
        let covered: HashSet<u16> = wanted.rows.keys().copied().collect();
        let undo: Vec<u16> = moved.keys().copied().filter(|r| !covered.contains(r)).collect();
        let Ok(decks) = std::fs::read_dir(player.join("decks")) else { continue };
        report.players += 1;
        let mut files: Vec<PathBuf> = decks.filter_map(Result::ok).map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("y3d"))).collect();
        files.sort();
        let mut failed = false;
        for file in files {
            match adapt_file(&file, wanted, &mut moved) {
                Ok(0) => {}
                Ok(n) => { report.adapted += 1; report.moved += n }
                Err(why) => { failed = true; report.skipped.push((file, why)) }
            }
        }
        // Where the cards the profile does not cover now stand is the game's own again: forget them.
        for row in undo { moved.remove(&row); }
        // A run that could not finish leaves the marker as it was, so the next start tries again.
        if !failed { write_marker(&marker, &wanted.hash, &moved)? }
    }
    Ok(report)
}

/// One deck file placed; the number of cards moved (0: the file is not touched).
fn adapt_file(file: &Path, wanted: &Wanted, moved: &mut Moved) -> Result<usize, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("cannot be read: {e}"))?;
    let mut deck = decode_file(&bytes).map_err(|e| format!("is not a deck the game wrote: {e}"))?;
    let (Some(main), Some(extra)) = (rows(&deck, "main"), rows(&deck, "extra")) else { return Err("has no Main and Extra lists".into()) };
    let (new_main, new_extra) = place(&main, &extra, wanted, moved);
    if (new_main.as_slice(), new_extra.as_slice()) == (main.as_slice(), extra.as_slice()) { return Ok(0) }
    let count = new_extra.iter().filter(|r| !extra.contains(r)).count() + new_main.iter().filter(|r| !main.contains(r)).count();
    set_rows(&mut deck, "main", &new_main);
    set_rows(&mut deck, "extra", &new_extra);
    let backup = PathBuf::from(format!("{}{BACKUP}", file.display()));
    if !backup.exists() { std::fs::copy(file, &backup).map_err(|e| format!("could not be backed up: {e}"))?; }
    write_atomic(file, &frame(&deck.encode())).map_err(|e| format!("could not be written: {e}"))?;
    Ok(count.max(1))
}

/// The game's `Yu-Gi-Oh! ONLINE 3` folder for the server with this game data id, given the original one: the original
/// folder when it is unclaimed or claimed by this id (`revival-server.txt`, tools/mod/documents.h), else
/// `<Documents>/YO3 Revival/servers/<id>/Yu-Gi-Oh! ONLINE 3`.
pub fn folder_for(original: &Path, server_id: &str) -> PathBuf {
    let owner = std::fs::read_to_string(original.join("revival-server.txt")).map(|t| t.trim().to_string()).unwrap_or_default();
    if owner.is_empty() || owner == server_id { return original.to_path_buf() }
    let documents = original.parent().unwrap_or(original);
    documents.join("YO3 Revival").join("servers").join(server_id).join("Yu-Gi-Oh! ONLINE 3")
}

/// The profile file as the game will get it, asked of the launcher's own local relay (`port`, under the launch cookie).
pub async fn fetch(port: u16, cookie: &str) -> anyhow::Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::time::timeout(std::time::Duration::from_secs(5), tokio::net::TcpStream::connect(("127.0.0.1", port))).await??;
    stream.write_all(format!("GET /session/{cookie}/web/card-profile?for=decks HTTP/1.0\r\nHost: localhost\r\n\r\n").as_bytes()).await?;
    let mut reply = vec![];
    tokio::time::timeout(std::time::Duration::from_secs(10), stream.take(4 << 20).read_to_end(&mut reply)).await??;
    let text = String::from_utf8_lossy(&reply).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").ok_or_else(|| anyhow::anyhow!("no answer"))?;
    anyhow::ensure!(head.starts_with("HTTP/1.1 200") || head.starts_with("HTTP/1.0 200"), "the server answered: {}", head.lines().next().unwrap_or(""));
    Ok(body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck(main: &[u16], extra: &[u16]) -> Vec<u8> {
        let message = Message::deck().with("deck_id", Value::U32(1)).with("deck_name", Value::text("Test"))
            .with("main", Value::u16_be_array(main)).with("extra", Value::u16_be_array(extra)).with("side", Value::u16_be_array(&[]));
        frame(&message.encode())
    }

    fn lists(file: &Path) -> (Vec<u16>, Vec<u16>) {
        let message = decode_file(&std::fs::read(file).unwrap()).unwrap();
        (rows(&message, "main").unwrap(), rows(&message, "extra").unwrap())
    }

    fn profile(rows: &[(u16, u32)], hash: &str) -> String {
        let mut text = format!("yo3-card-profile 1\nprofile anime\nhash {hash}\n");
        for (row, kind) in rows { text += &format!("card {row} 4021 00000000 {:08x} 0000000000000000 - -\n", kind << 3) }
        text
    }

    /// A Documents folder with one player and these decks.
    fn fixture(decks: &[(&str, Vec<u8>)]) -> tempdir::Dir {
        let dir = tempdir::Dir::new();
        let folder = dir.path().join("players/Duelist/decks");
        std::fs::create_dir_all(&folder).unwrap();
        for (name, bytes) in decks { std::fs::write(folder.join(name), bytes).unwrap() }
        dir
    }

    mod tempdir {
        pub struct Dir(std::path::PathBuf);
        impl Dir {
            pub fn new() -> Dir {
                static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                let path = std::env::temp_dir().join(format!("yo3-decks-test-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
                std::fs::create_dir_all(&path).unwrap();
                Dir(path)
            }
            pub fn path(&self) -> &std::path::Path { &self.0 }
        }
        impl Drop for Dir { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
    }

    #[test]
    fn the_profile_file_says_which_cards_belong_where() {
        // Row 101 is a Normal Monster (kind 0) and row 7 a Fusion Monster (kind 3) in the profile.
        let wanted = Wanted::parse(&profile(&[(101, 0), (7, 3), (8, 18)], "abc"));
        assert_eq!(wanted.hash, "abc");
        assert_eq!(wanted.rows, BTreeMap::from([(7, true), (8, true), (101, false)]));
        assert_eq!(Wanted::parse(""), Wanted { hash: "official".into(), rows: BTreeMap::new() });
        assert_eq!(Wanted::parse("not a profile\ncard 1 2 3 4 5"), Wanted { hash: "official".into(), rows: BTreeMap::new() });
    }

    #[test]
    fn flame_swordsman_moves_to_the_main_deck_under_the_profile_and_back_under_the_games_own() {
        let original = deck(&[10, 20, 20], &[101, 101, 50]);
        let dir = fixture(&[("Flame.y3d", original.clone()), ("Plain.y3d", deck(&[10, 11], &[50]))]);
        let file = dir.path().join("players/Duelist/decks/Flame.y3d");
        // The anime profile: row 101 (Flame Swordsman) is a Normal Monster.
        let anime = Wanted::parse(&profile(&[(101, 0)], "anime1"));
        let report = adapt(dir.path(), &anime).unwrap();
        assert_eq!((report.players, report.adapted, report.moved), (1, 1, 2), "{report:?}");
        assert_eq!(lists(&file), (vec![10, 20, 20, 101, 101], vec![50]), "quantities stay, the Fusion Monster's copies join the main list");
        assert_eq!(lists(&dir.path().join("players/Duelist/decks/Plain.y3d")), (vec![10, 11], vec![50]), "an untouched deck stays byte for byte");
        let backup = dir.path().join("players/Duelist/decks/Flame.y3d.pre-profile");
        assert_eq!(std::fs::read(&backup).unwrap(), original, "the game's own file is kept once");
        assert!(!dir.path().join("players/Duelist/decks/Plain.y3d.pre-profile").exists(), "no backup of a file that did not change");
        let marker = std::fs::read_to_string(dir.path().join("players/Duelist").join(MARKER)).unwrap();
        assert_eq!(marker, "hash anime1\nmoved 101 extra\n");
        // The same profile again: nothing happens.
        let again = adapt(dir.path(), &anime).unwrap();
        assert_eq!((again.players, again.adapted), (0, 0));
        // The game's own profile (covers nothing): the cards the profile moved go back to where the game has them.
        let report = adapt(dir.path(), &Wanted::parse("")).unwrap();
        assert_eq!((report.adapted, report.moved), (1, 2), "{report:?}");
        assert_eq!(lists(&file), (vec![10, 20, 20], vec![50, 101, 101]));
        assert_eq!(std::fs::read(&backup).unwrap(), original, "the backup is never overwritten");
        assert_eq!(std::fs::read_to_string(dir.path().join("players/Duelist").join(MARKER)).unwrap(), "hash official\n");
    }

    #[test]
    fn switching_the_profile_off_and_on_again_keeps_every_card_and_quantity_of_every_deck() {
        // Rows: 101 and 102 are Flame Swordsman-like cards the profile moves (Extra in the game, Main in the profile),
        // 10-12 and 50 are cards it never touches. Three decks, copies of each, and a side deck that is never touched.
        let decks = [
            ("A.y3d", (vec![10, 11, 11, 12], vec![101, 101, 101, 50])),
            ("B.y3d", (vec![101, 10], vec![102, 50, 50])),
            ("C.y3d", (vec![10, 10, 10], vec![])),
        ];
        let dir = fixture(&decks.iter().map(|(n, (m, e))| (*n, deck(m, e))).collect::<Vec<_>>());
        let folder = dir.path().join("players/Duelist/decks");
        let counts = |file: &str| {
            let (main, extra) = lists(&folder.join(file));
            let mut all: BTreeMap<u16, usize> = BTreeMap::new();
            for row in main.iter().chain(&extra) { *all.entry(*row).or_default() += 1 }
            all
        };
        let originals: Vec<_> = decks.iter().map(|(n, _)| counts(n)).collect();
        let anime = |hash: &str| Wanted::parse(&profile(&[(101, 0), (102, 0)], hash));
        for round in 0..3 {
            adapt(dir.path(), &anime(&format!("on{round}"))).unwrap();
            for (i, (name, _)) in decks.iter().enumerate() {
                assert_eq!(counts(name), originals[i], "{name}: on, round {round}: every card and quantity is still there");
                let (_, extra) = lists(&folder.join(name));
                assert!(!extra.contains(&101) && !extra.contains(&102), "{name}: the profile's cards are in the main list");
            }
            adapt(dir.path(), &Wanted::parse("")).unwrap();
            for (i, (name, (main, extra))) in decks.iter().enumerate() {
                assert_eq!(counts(name), originals[i], "{name}: off, round {round}");
                let (mut m, mut e) = lists(&folder.join(name));
                assert!(!m.contains(&101) && !m.contains(&102), "{name}: off, round {round}: the game keeps them in the Extra Deck");
                if *name == "B.y3d" { continue } // its Main Deck copy of 101 was the player's own and is a game's Extra Deck card again
                let (mut main, mut extra) = (main.clone(), extra.clone());
                for v in [&mut m, &mut e, &mut main, &mut extra] { v.sort() }
                assert_eq!((m, e), (main, extra), "{name}: off, round {round}: exactly as the game had it");
            }
        }
    }

    #[test]
    fn a_card_the_player_put_where_the_profile_wants_it_is_not_recorded_as_moved() {
        let dir = fixture(&[("Mine.y3d", deck(&[101, 1], &[]))]);
        adapt(dir.path(), &Wanted::parse(&profile(&[(101, 0)], "h"))).unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("players/Duelist").join(MARKER)).unwrap(), "hash h\n");
        assert!(!dir.path().join("players/Duelist/decks/Mine.y3d.pre-profile").exists());
        // Going back to the game's own profile leaves it in the Main Deck: nothing was moved for it.
        adapt(dir.path(), &Wanted::parse("")).unwrap();
        assert_eq!(lists(&dir.path().join("players/Duelist/decks/Mine.y3d")), (vec![101, 1], vec![]));
    }

    #[test]
    fn files_that_are_not_decks_are_left_alone_and_named() {
        let garbage = b"this is not a deck at all".to_vec();
        let dir = fixture(&[("Broken.y3d", garbage.clone()), ("Flame.y3d", deck(&[], &[101])), ("notes.txt", b"hi".to_vec())]);
        let report = adapt(dir.path(), &Wanted::parse(&profile(&[(101, 0)], "h"))).unwrap();
        assert_eq!((report.adapted, report.moved, report.skipped.len()), (1, 1, 1), "{report:?}");
        assert!(report.skipped[0].0.ends_with("Broken.y3d"));
        let decks = dir.path().join("players/Duelist/decks");
        assert_eq!(std::fs::read(decks.join("Broken.y3d")).unwrap(), garbage);
        assert!(!decks.join("Broken.y3d.pre-profile").exists());
        assert_eq!(std::fs::read(decks.join("notes.txt")).unwrap(), b"hi");
        assert_eq!(lists(&decks.join("Flame.y3d")), (vec![101], vec![]));
        // The marker is not written after a file failed, so the next start tries again.
        assert!(!dir.path().join("players/Duelist").join(MARKER).exists());
    }

    #[test]
    fn the_stale_marker_follows_what_the_server_pins_against_what_the_game_loaded() {
        let dir = tempdir::Dir::new();
        let ready = dir.path().join("game-ready.json");
        let anime = Wanted::parse(&profile(&[(101, 0)], "anime1"));
        assert_eq!((anime.announced(), Wanted::parse("").announced()), ("anime1".to_string(), String::new()));
        assert!(!note_staleness(&ready, &anime.announced(), "anime1") && !stale_file(&ready).exists(), "the same profile");
        assert!(note_staleness(&ready, &anime.announced(), ""), "the host turned the profile off");
        assert!(stale_file(&ready).exists());
        assert!(note_staleness(&ready, "", "anime2") && stale_file(&ready).exists(), "or turned another on");
        assert!(!note_staleness(&ready, "", "") && !stale_file(&ready).exists(), "and back: the game is current again");
    }

    /// The profile is asked of the launcher's own relay under the launch cookie, as the game asks it.
    #[tokio::test]
    async fn the_profile_is_fetched_from_the_relay_under_the_launch_cookie() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let body = profile(&[(101, 0)], "anime1");
        let served = body.clone();
        let relay = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 512];
            let n = stream.read(&mut request).await.unwrap();
            let line = String::from_utf8_lossy(&request[..n]).lines().next().unwrap().to_string();
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{served}", served.len()).as_bytes()).await.unwrap();
            line
        });
        let fetched = fetch(port, "c00kie").await.unwrap();
        assert_eq!(relay.await.unwrap(), "GET /session/c00kie/web/card-profile?for=decks HTTP/1.0");
        assert_eq!(fetched, body);
        assert_eq!(Wanted::parse(&fetched).rows, BTreeMap::from([(101, false)]));
    }

    #[test]
    fn the_folder_is_the_originals_unless_another_server_claimed_it() {
        let dir = tempdir::Dir::new();
        let original = dir.path().join("Documents/Yu-Gi-Oh! ONLINE 3");
        std::fs::create_dir_all(&original).unwrap();
        assert_eq!(folder_for(&original, "aaaa"), original, "unclaimed");
        std::fs::write(original.join("revival-server.txt"), "aaaa\n").unwrap();
        assert_eq!(folder_for(&original, "aaaa"), original, "claimed by this server");
        assert_eq!(folder_for(&original, "bbbb"), dir.path().join("Documents/YO3 Revival/servers/bbbb/Yu-Gi-Oh! ONLINE 3"));
    }
}
