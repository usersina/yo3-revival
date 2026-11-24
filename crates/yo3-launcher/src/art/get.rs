//! Getting the anime pictures from ProjectIgnis' picture server (issue #60), on the player's click only. The project hosts
//! no picture: the list below says which passcode to ask for per card, the [`Fetcher`] asks (Desktop's does it over HTTPS,
//! the tests' does not touch a network), and each picture is checked, made 200x290 and written as the folder's own file.
//!
//! * One card at a time, [`PACE`] apart, [`TRIES`] tries each (a failed request or a picture that can't be read, waiting
//!   longer each time); cancelling stops at the next card. A card whose face exists is
//!   skipped, so a second run only finishes the missing ones, and a face the player put there is never replaced.
//! * A face is written whole or not at all (a temporary name, then renamed), so the game, which reads the folder at its
//!   start, never meets a half-written picture. A card without a face, for any reason (not fetched yet, cancelled, failed,
//!   refused as no card picture), shows the game's own artwork in the game, with the profile's stats and text unchanged.
//! * The faces this made are listed in `card/.downloaded`: a face not listed is the player's own.
use super::{FACE, decode_png, id_of, write_png, prepare};
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// The wait between two requests to the picture server.
pub const PACE: Duration = Duration::from_millis(400);
/// What the whole set weighs, said before the player clicks (177x254 JPEGs, about 25 KB each).
pub const DOWNLOAD_SIZE: &str = "about 10 MB";
/// A picture larger than this is refused unread.
pub const MAX_BYTES: usize = 2 << 20;
/// Faces made by Desktop's download, in `card/`.
pub const DOWNLOADED: &str = ".downloaded";
/// How many times a picture is asked for before the card is given up: a timeout, a refused request or a picture that
/// arrived broken is often gone on the next try.
pub const TRIES: u32 = 3;
/// After this many cards in a row that the server could not be asked about, the run stops: the server is out of reach.
const GIVE_UP: u32 = 3;

/// One card of the list: the Konami id, the passcode of its anime picture (0: none) and the official passcode whose normal
/// picture stands in when the server has no anime one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry { pub cid: u32, pub anime: u32, pub official: u32 }

/// The list (`engine/anime/pictures.tsv`, built by `tools/engine/anime-pictures.py` from the reviewed map).
pub fn list() -> Vec<Entry> { parse(include_str!("../../../../engine/anime/pictures.tsv")) }

fn parse(text: &str) -> Vec<Entry> {
    text.lines().skip(1).filter_map(|line| {
        let mut f = line.split('\t').map(|v| v.parse::<u32>());
        Some(Entry { cid: f.next()?.ok()?, anime: f.next()?.ok()?, official: f.next()?.ok()? })
    }).collect()
}

/// What the server said for one passcode.
#[derive(Debug, PartialEq, Eq)]
pub enum Fetched { Jpeg(Vec<u8>), Missing }

/// Asks for pictures. An `Err` is a problem reaching the server (worth a retry); a picture the server does not have is
/// `Ok(Fetched::Missing)`.
pub trait Fetcher { fn fetch(&mut self, passcode: u32) -> Result<Fetched, String>; }

/// A downloaded JPEG as a 200x290 RGBA face, or why it is no card picture. Sizes are read before anything is decoded.
pub fn face_from_jpeg(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() > MAX_BYTES { return Err("the picture is too large".into()) }
    if !bytes.starts_with(&[0xFF, 0xD8, 0xFF]) { return Err("it is not a JPEG picture".into()) }
    let mut decoder = jpeg_decoder::Decoder::new(io::Cursor::new(bytes));
    decoder.read_info().map_err(|e| e.to_string())?;
    let info = decoder.info().ok_or("the picture has no size")?;
    let (w, h) = (u32::from(info.width), u32::from(info.height));
    if !(32..=2048).contains(&w) || !(32..=2048).contains(&h) { return Err(format!("its size, {w}x{h}, is not a card's")) }
    if (w as f64 / h as f64 - FACE.0 as f64 / FACE.1 as f64).abs() > 0.03 { return Err(format!("its shape, {w}x{h}, is not a card's")) }
    let pixels = decoder.decode().map_err(|e| e.to_string())?;
    let rgba: Vec<u8> = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => pixels.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        jpeg_decoder::PixelFormat::L8 => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        _ => return Err("its colours are not supported".into()),
    };
    if rgba.len() != (w * h * 4) as usize { return Err("the picture is damaged".into()) }
    Ok(resize(&rgba, (w, h), FACE))
}

/// `rgba` (w x h, opaque) scaled to `to`: larger pictures are averaged down like the thumbnails, smaller ones are enlarged with
/// bilinear interpolation (pixel centres lined up, so the corners stay the corners).
pub fn resize(rgba: &[u8], (w, h): (u32, u32), to: (u32, u32)) -> Vec<u8> {
    if to.0 <= w && to.1 <= h { return super::downscale(rgba, (w, h), to) }
    let at = |x: u32, y: u32, c: usize| f32::from(rgba[((y * w + x) * 4) as usize + c]);
    let mut out = Vec::with_capacity((to.0 * to.1 * 4) as usize);
    for y in 0..to.1 {
        let sy = ((y as f32 + 0.5) * h as f32 / to.1 as f32 - 0.5).clamp(0.0, (h - 1) as f32);
        let (y0, fy) = (sy.floor() as u32, sy.fract());
        let y1 = (y0 + 1).min(h - 1);
        for x in 0..to.0 {
            let sx = ((x as f32 + 0.5) * w as f32 / to.0 as f32 - 0.5).clamp(0.0, (w - 1) as f32);
            let (x0, fx) = (sx.floor() as u32, sx.fract());
            let x1 = (x0 + 1).min(w - 1);
            for c in 0..4 {
                let top = at(x0, y0, c) * (1.0 - fx) + at(x1, y0, c) * fx;
                let bottom = at(x0, y1, c) * (1.0 - fx) + at(x1, y1, c) * fx;
                out.push((top * (1.0 - fy) + bottom * fy).round() as u8);
            }
        }
    }
    out
}

/// The faces Desktop made (`card/.downloaded`).
pub fn downloaded(dir: &Path) -> BTreeSet<u32> {
    std::fs::read_to_string(dir.join("card").join(DOWNLOADED)).unwrap_or_default().lines().filter_map(|l| l.trim().parse().ok()).collect()
}

pub(crate) fn save_ids(file: &Path, ids: &BTreeSet<u32>) -> io::Result<()> {
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, ids.iter().map(|i| format!("{i}\n")).collect::<String>())?;
    std::fs::rename(&tmp, file)
}

/// How many of `entries` have a usable face in the folder (a 200x290 PNG).
pub fn have(dir: &Path, entries: &[Entry]) -> usize {
    let faces: BTreeSet<u32> = std::fs::read_dir(dir.join("card")).into_iter().flatten().filter_map(Result::ok).filter_map(|e| {
        let id = id_of(&e.path())?;
        let ok = std::fs::File::open(e.path()).ok().and_then(|f| decode_png(io::BufReader::new(f)).ok()).is_some_and(|(w, h, _)| (w, h) == FACE);
        ok.then_some(id)
    }).collect();
    entries.iter().filter(|e| faces.contains(&e.cid)).count()
}

/// What a run did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Faces written by this run.
    pub saved: Vec<u32>,
    /// Cards that already had a face (the player's own, or from an earlier run): left alone.
    pub had: u32,
    /// Cards that got no picture, with why: the game keeps its own artwork for them.
    pub failed: Vec<(u32, String)>,
    /// The player cancelled.
    pub cancelled: bool,
    /// The server could not be reached, so the run stopped (what the last request said).
    pub unreachable: Option<String>,
}

/// Fetches the pictures of `entries` that have no face yet, into `dir` (see the module's rules). `progress(done, total)` is called
/// after every card. The thumbnails follow from the faces when the run ends.
pub fn download(dir: &Path, entries: &[Entry], fetcher: &mut dyn Fetcher, pace: Duration, cancel: &AtomicBool,
    progress: &mut dyn FnMut(u32, u32)) -> io::Result<Outcome> {
    std::fs::create_dir_all(dir.join("card"))?;
    let marks = dir.join("card").join(DOWNLOADED);
    let mut ours = downloaded(dir);
    let (mut out, total, mut asked, mut lost) = (Outcome::default(), entries.len() as u32, false, 0);
    for (n, entry) in entries.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) { out.cancelled = true; break }
        let face: PathBuf = dir.join("card").join(format!("{}.png", entry.cid));
        if face.exists() { out.had += 1; progress(n as u32 + 1, total); continue }
        // A picture or the server's "none" (Ok), or why it couldn't be had after TRIES tries: Err(network?, why).
        let mut ask = |passcode: u32| -> Result<Option<Vec<u8>>, (bool, String)> {
            let mut last = (true, String::new());
            for attempt in 0..TRIES {
                if asked { std::thread::sleep(if attempt == 0 { pace } else { pace * 5 * attempt }) }
                asked = true;
                match fetcher.fetch(passcode) {
                    Ok(Fetched::Missing) => return Ok(None),
                    Ok(Fetched::Jpeg(bytes)) => match face_from_jpeg(&bytes) {
                        Ok(rgba) => return Ok(Some(rgba)),
                        Err(why) => last = (false, why),
                    },
                    Err(why) => last = (true, why),
                }
            }
            Err(last)
        };
        let mut found = if entry.anime != 0 { ask(entry.anime) } else { Ok(None) };
        if matches!(found, Ok(None)) && entry.official != 0 { found = ask(entry.official) }
        match found {
            Ok(Some(rgba)) => {
                lost = 0;
                write_png(&face, FACE, &rgba)?;
                ours.insert(entry.cid);
                save_ids(&marks, &ours)?;
                out.saved.push(entry.cid);
            }
            Ok(None) => { lost = 0; out.failed.push((entry.cid, "the server has no picture of it".into())) }
            Err((false, why)) => { lost = 0; out.failed.push((entry.cid, why)) }
            Err((true, why)) => {
                lost += 1;
                out.failed.push((entry.cid, why.clone()));
                if lost >= GIVE_UP { out.unreachable = Some(why); progress(n as u32 + 1, total); break }
            }
        }
        progress(n as u32 + 1, total);
    }
    prepare(dir)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A JPEG of a generated gradient with a red corner (never a card), `w` x `h`.
    fn gradient(w: u16, h: u16) -> Vec<u8> {
        let rgb: Vec<u8> = (0..u32::from(w) * u32::from(h)).flat_map(|i| {
            let (x, y) = (i % u32::from(w), i / u32::from(w));
            if x < 12 && y < 12 { [200, 30, 30] } else { [(x * 255 / u32::from(w)) as u8, (y * 255 / u32::from(h)) as u8, 128] }
        }).collect();
        let mut out = Vec::new();
        jpeg_encoder::Encoder::new(&mut out, 90).encode(&rgb, w, h, jpeg_encoder::ColorType::Rgb).unwrap();
        out
    }
    fn jpeg_177x254() -> Vec<u8> { gradient(177, 254) }
    fn jpeg_square() -> Vec<u8> { gradient(200, 200) }

    #[derive(Default)]
    struct Fake { pics: BTreeMap<u32, Result<Fetched, String>>, asked: Vec<u32>, failing: u32 }
    impl Fetcher for Fake {
        fn fetch(&mut self, passcode: u32) -> Result<Fetched, String> {
            self.asked.push(passcode);
            if self.failing > 0 { self.failing -= 1; return Err("no connection".into()) }
            match self.pics.get(&passcode) {
                Some(Ok(Fetched::Jpeg(b))) => Ok(Fetched::Jpeg(b.clone())),
                Some(Err(e)) => Err(e.clone()),
                _ => Ok(Fetched::Missing),
            }
        }
    }
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yo3-art-get-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }
    fn run(dir: &Path, entries: &[Entry], fake: &mut Fake) -> Outcome {
        download(dir, entries, fake, Duration::ZERO, &AtomicBool::new(false), &mut |_, _| {}).unwrap()
    }
    fn jpeg() -> Fetched { Fetched::Jpeg(jpeg_177x254()) }

    #[test]
    fn the_list_has_every_card_of_the_map_once() {
        // The 418 anime cards and the cards the game never had (#75: Quasar, which has an anime version, and Orichalcos
        // Gigas), each fetched by its official passcode, an anime-only card by its anime one.
        let all = list();
        assert!(all.len() >= 420, "the 418 anime cards and the added cards");
        assert_eq!(all.iter().map(|e| e.cid).collect::<BTreeSet<_>>().len(), all.len(), "each card once");
        assert_eq!(all.iter().find(|e| e.cid == 3968), Some(&Entry { cid: 3968, anime: 170000171, official: 170000171 }), "Orichalcos Gigas, anime-only");
        assert_eq!(all.iter().find(|e| e.cid == 9709), Some(&Entry { cid: 9709, anime: 513000031, official: 35952884 }), "Shooting Quasar Dragon, an added card with an anime version");
        assert!(all.iter().all(|e| e.official != 0));
        assert_eq!(all.iter().find(|e| e.cid == 5231), Some(&Entry { cid: 5231, anime: 0, official: 29762407 }));
        assert_eq!(all.iter().find(|e| e.cid == 4998).map(|e| e.anime), Some(511600398), "a God is fetched by its anime passcode like any card");
    }

    #[test]
    fn a_177x254_jpeg_becomes_a_200x290_face_and_the_corners_stay_put() {
        let rgba = face_from_jpeg(&jpeg_177x254()).unwrap();
        assert_eq!(rgba.len(), (FACE.0 * FACE.1 * 4) as usize);
        assert!(rgba[0] > 150 && rgba[1] < 120, "the red corner is still the top-left: {:?}", &rgba[..4]);
        let last = &rgba[rgba.len() - 4..];
        assert!(last[0] < 120 || last[2] > 100, "the far corner is the gradient's: {last:?}");
    }

    #[test]
    fn the_resize_enlarges_smoothly_and_averages_down() {
        // A 2x2 picture enlarged to 4x4: the corners are exact, the middle blends the neighbours.
        let small = [0, 0, 0, 255, 200, 0, 0, 255, 0, 200, 0, 255, 200, 200, 0, 255];
        let big = resize(&small, (2, 2), (4, 4));
        assert_eq!(big.len(), 64);
        assert_eq!(&big[..4], &[0, 0, 0, 255]);
        assert_eq!(&big[60..64], &[200, 200, 0, 255]);
        let mid = &big[(4 + 1) * 4..][..4];
        assert!(mid[0] > 20 && mid[0] < 180, "{mid:?}");
        // Larger to smaller is the thumbnails' box filter.
        let white = vec![255u8; 8 * 8 * 4];
        assert_eq!(resize(&white, (8, 8), (4, 4)), vec![255u8; 4 * 4 * 4]);
    }

    #[test]
    fn a_picture_that_is_no_card_is_refused_before_it_is_decoded() {
        assert!(face_from_jpeg(&jpeg_square()).unwrap_err().contains("shape"));
        assert!(face_from_jpeg(b"<html>Not found</html>").unwrap_err().contains("not a JPEG"));
        assert!(face_from_jpeg(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0]).is_err());
        assert!(face_from_jpeg(&vec![0xFF; MAX_BYTES + 1]).unwrap_err().contains("too large"));
    }

    #[test]
    fn the_anime_picture_is_asked_for_first_and_a_404_falls_back_to_the_official_one() {
        let dir = scratch("fallback");
        let entries = [Entry { cid: 10, anime: 1000, official: 1 }, Entry { cid: 11, anime: 1100, official: 2 }, Entry { cid: 12, anime: 0, official: 3 }];
        let mut fake = Fake::default();
        fake.pics.insert(1000, Ok(jpeg()));
        fake.pics.insert(2, Ok(jpeg()));
        fake.pics.insert(3, Ok(jpeg()));
        let out = run(&dir, &entries, &mut fake);
        assert_eq!(out.saved, vec![10, 11, 12]);
        assert_eq!(fake.asked, vec![1000, 1100, 2, 3], "the anime passcode, then on a 404 the official one; none: the official one");
        assert_eq!(super::super::size_of(&dir.join("card/11.png")).unwrap(), FACE);
        assert_eq!(super::super::size_of(&dir.join("mini/11.png")).unwrap(), super::super::MINI, "the thumbnail follows");
        assert_eq!(downloaded(&dir), BTreeSet::from([10, 11, 12]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_picture_the_player_provided_is_never_replaced_and_a_second_run_only_finishes_the_rest() {
        let dir = scratch("never");
        std::fs::create_dir_all(dir.join("card")).unwrap();
        let own = vec![9u8; 40];
        std::fs::write(dir.join("card/10.png"), &own).unwrap();
        let entries = [Entry { cid: 10, anime: 1000, official: 1 }, Entry { cid: 11, anime: 1100, official: 2 }];
        let mut fake = Fake::default();
        fake.pics.insert(1000, Ok(jpeg()));
        fake.pics.insert(1100, Err("timed out".into()));
        let first = run(&dir, &entries, &mut fake);
        assert_eq!((first.had, first.saved.clone()), (1, vec![]));
        assert_eq!(std::fs::read(dir.join("card/10.png")).unwrap(), own, "the player's own picture is untouched");
        assert!(!dir.join("card/11.png").exists(), "a card whose request failed has no picture, no empty file");
        // The server answers now: only the missing card is asked for.
        fake.pics.insert(1100, Ok(jpeg()));
        fake.asked.clear();
        let second = run(&dir, &entries, &mut fake);
        assert_eq!((second.saved, second.had), (vec![11], 1));
        assert_eq!(fake.asked, vec![1100]);
        assert_eq!(downloaded(&dir), BTreeSet::from([11]), "only what Desktop made is listed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_partial_download_leaves_whole_pictures_and_cards_that_keep_the_games_art() {
        // Some cards fetched, one refused (not a card picture), one missing on the server, one cancelled before it: the folder
        // holds only complete 200x290 faces and no temporary or empty file, so the game shows the game's own artwork for every
        // other card (the DLL falls back for a card without a file), never a blank or broken picture.
        let dir = scratch("partial");
        let entries: Vec<Entry> = (1..=5).map(|i| Entry { cid: i, anime: 100 + i, official: 200 + i }).collect();
        let mut fake = Fake::default();
        fake.pics.insert(101, Ok(jpeg()));
        fake.pics.insert(102, Ok(Fetched::Jpeg(jpeg_square())));
        fake.pics.insert(104, Ok(jpeg()));
        let cancel = AtomicBool::new(false);
        let out = download(&dir, &entries, &mut fake, Duration::ZERO, &cancel, &mut |done, _| if done == 4 { cancel.store(true, Ordering::SeqCst) }).unwrap();
        assert!(out.cancelled);
        assert_eq!(out.saved, vec![1, 4]);
        assert_eq!(out.failed.iter().map(|f| f.0).collect::<Vec<_>>(), vec![2, 3]);
        let mut names: Vec<String> = std::fs::read_dir(dir.join("card")).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, vec![".downloaded", "1.png", "4.png"], "no half-written or empty file");
        assert_eq!(have(&dir, &entries), 2);
        let prepared = prepare(&dir).unwrap();
        assert_eq!(super::super::coverage(&prepared, &entries.iter().map(|e| e.cid).collect()), (vec![1, 4], vec![2, 3, 5]));
        assert!(prepared.skipped.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_request_is_tried_three_times_and_a_server_out_of_reach_stops_the_run() {
        let dir = scratch("unreachable");
        let entries: Vec<Entry> = (1..=8).map(|i| Entry { cid: i, anime: 100 + i, official: 0 }).collect();
        let mut fake = Fake { failing: 2, ..Fake::default() };
        fake.pics.insert(101, Ok(jpeg()));
        let one = run(&dir, &entries[..1], &mut fake);
        assert_eq!((one.saved, fake.asked.len()), (vec![1], 3), "two tries failed (a timeout, an invalid request), the third got it");
        let mut down = Fake { failing: 1000, ..Fake::default() };
        let out = run(&dir, &entries[1..], &mut down);
        assert_eq!(out.unreachable.as_deref(), Some("no connection"));
        assert_eq!(down.asked.len(), 9, "three cards, three tries each, then it stops");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
