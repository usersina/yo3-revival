//! Pictures by passcode, as EDOPro takes them (`pics/<passcode>.jpg` or `.png`): a player drops a card's picture into the
//! `pics` folder (beside `anime-art` in the launcher's data folder, or beside `yo3.exe`) and the launcher makes it the game's
//! face and thumbnail at each start, for every card the picture list knows (`engine/anime/pictures.tsv`): the cards shown
//! as their anime or manga versions and the cards the game never had. A card is matched by its anime passcode first, then
//! its official one, so EDOPro's own `pics` folder works as it is.
//!
//! The project also ships pictures of its own ([`SHIPPED`], made by the owner, `crates/yo3-launcher/assets/pics/`), used
//! the same way. The order, first wins: the player's `pics` folders, a face the player put in `anime-art/card` by hand,
//! the shipped pictures, a downloaded picture. The faces made here are listed in `card/.pics` with where each came from, so a
//! changed picture is made again and the face of one taken away is removed (and downloaded again if the player asks).
use super::get::{DOWNLOADED, Entry, downloaded, resize, save_ids};
use super::{FACE, id_of, write_png};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

/// The folder's name, beside `anime-art`.
pub const FOLDER: &str = "pics";
/// The faces made from pictures, in `card/`: `<Konami id>\t<source>` per line.
pub const MADE: &str = ".pics";
/// A picture larger than this is refused unread (EDOPro's high-resolution card pictures are a few MB).
pub const MAX_BYTES: u64 = 24 << 20;
/// The largest side a picture may have.
const MAX_SIDE: u32 = 4096;

/// The pictures the project ships (passcode, the face as a 200x290 PNG): the owner's own artwork.
pub const SHIPPED: &[(u32, &[u8])] = &[
    (513000134, include_bytes!("../../assets/pics/513000134.png")), // The Winged Dragon of Ra (manga)
    (513000135, include_bytes!("../../assets/pics/513000135.png")), // Obelisk the Tormentor (manga)
    (513000136, include_bytes!("../../assets/pics/513000136.png")), // Slifer the Sky Dragon (manga)
    (513000137, include_bytes!("../../assets/pics/513000137.png")), // The Wicked Dreadroot (manga)
    (513000138, include_bytes!("../../assets/pics/513000138.png")), // The Wicked Avatar (manga)
    (513000139, include_bytes!("../../assets/pics/513000139.png")), // The Wicked Eraser (manga)
    (170000170, include_bytes!("../../assets/pics/170000170.png")), // Divine Serpent Geh (anime)
];

/// What [`apply`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Applied {
    /// Faces made now (Konami ids).
    pub made: Vec<u32>,
    /// Faces removed because their picture is gone.
    pub removed: Vec<u32>,
    /// Pictures refused, with why: the card keeps whatever face it had.
    pub refused: Vec<(PathBuf, String)>,
}

/// Where a face comes from: a player's file (its path, size and time, so a change is seen) or a shipped picture.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Source { File(PathBuf, String), Shipped(u32) }

impl Source {
    fn label(&self) -> String {
        match self { Source::File(path, stamp) => format!("{}|{stamp}", path.display()), Source::Shipped(code) => format!("shipped:{code}") }
    }
}

/// `<passcode>.png`, `.jpg` or `.jpeg` (any case) in `dir`.
fn file_for(dir: &Path, passcode: u32) -> Option<PathBuf> {
    ["png", "jpg", "jpeg", "PNG", "JPG", "JPEG"].iter().map(|ext| dir.join(format!("{passcode}.{ext}"))).find(|p| p.is_file())
}

fn stamp(path: &Path) -> String {
    let meta = std::fs::metadata(path).ok();
    let time = meta.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    format!("{}:{time}", meta.map_or(0, |m| m.len()))
}

/// The source for `entry`: the first player folder with its anime or official passcode, else a shipped picture.
fn source(entry: &Entry, folders: &[&Path]) -> Option<Source> {
    let codes: Vec<u32> = [entry.anime, entry.official].into_iter().filter(|&c| c != 0).collect();
    for dir in folders {
        for &code in &codes {
            if let Some(path) = file_for(dir, code) {
                let s = stamp(&path);
                return Some(Source::File(path, s));
            }
        }
    }
    codes.iter().find(|&&c| SHIPPED.iter().any(|(s, _)| *s == c)).map(|&c| Source::Shipped(c))
}

/// A picture (PNG or JPEG, any size up to [`MAX_SIDE`], the shape of a card) as the game's 200x290 face.
pub fn face_from_picture(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() as u64 > MAX_BYTES { return Err("the picture is too large".into()) }
    let (w, h, rgba) = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let mut decoder = png::Decoder::new(io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let (w, h) = (reader.info().width, reader.info().height);
        check_size(w, h)?;
        let mut raw = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut raw).map_err(|e| e.to_string())?;
        raw.truncate(info.buffer_size());
        let rgba = match info.color_type {
            png::ColorType::Rgba => raw,
            png::ColorType::Rgb => raw.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
            png::ColorType::GrayscaleAlpha => raw.chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
            png::ColorType::Grayscale => raw.iter().flat_map(|&g| [g, g, g, 255]).collect(),
            png::ColorType::Indexed => return Err("indexed colours were not expanded".into()),
        };
        (w, h, rgba)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        let mut decoder = jpeg_decoder::Decoder::new(io::Cursor::new(bytes));
        decoder.read_info().map_err(|e| e.to_string())?;
        let info = decoder.info().ok_or("the picture has no size")?;
        let (w, h) = (u32::from(info.width), u32::from(info.height));
        check_size(w, h)?;
        let pixels = decoder.decode().map_err(|e| e.to_string())?;
        let rgba: Vec<u8> = match info.pixel_format {
            jpeg_decoder::PixelFormat::RGB24 => pixels.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
            jpeg_decoder::PixelFormat::L8 => pixels.iter().flat_map(|&g| [g, g, g, 255]).collect(),
            _ => return Err("its colours are not supported".into()),
        };
        (w, h, rgba)
    } else {
        return Err("it is neither a PNG nor a JPEG picture".into());
    };
    if rgba.len() != (w * h * 4) as usize { return Err("the picture is damaged".into()) }
    Ok(resize(&rgba, (w, h), FACE))
}

/// A card's size and shape: EDOPro's pictures (177x254, 813x1185) and the game's faces (200x290) all are, within 3 %.
fn check_size(w: u32, h: u32) -> Result<(), String> {
    if !(32..=MAX_SIDE).contains(&w) || !(32..=MAX_SIDE).contains(&h) { return Err(format!("its size, {w}x{h}, is not a card's")) }
    if (w as f64 / h as f64 - FACE.0 as f64 / FACE.1 as f64).abs() > 0.03 { return Err(format!("its shape, {w}x{h}, is not a card's")) }
    Ok(())
}

fn read_made(file: &Path) -> BTreeMap<u32, String> {
    std::fs::read_to_string(file).unwrap_or_default().lines()
        .filter_map(|l| { let (id, src) = l.split_once('\t')?; Some((id.trim().parse().ok()?, src.to_string())) }).collect()
}

fn save_made(file: &Path, made: &BTreeMap<u32, String>) -> io::Result<()> {
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, made.iter().map(|(id, src)| format!("{id}\t{src}\n")).collect::<String>())?;
    std::fs::rename(&tmp, file)
}

/// Makes the faces in `art` (the `anime-art` folder) from the pictures in `folders` (in order) and the shipped ones, for the
/// cards of `entries`. Run before [`super::prepare`], which then makes their thumbnails. Never writes outside `art/card`.
pub fn apply(art: &Path, folders: &[&Path], entries: &[Entry]) -> io::Result<Applied> {
    let card = art.join("card");
    std::fs::create_dir_all(&card)?;
    let made_file = card.join(MADE);
    let mut made = read_made(&made_file);
    let mut fetched = downloaded(art);
    let mut done = Applied::default();
    let existing: BTreeSet<u32> = std::fs::read_dir(&card)?.filter_map(Result::ok).filter_map(|e| id_of(&e.path())).collect();
    for entry in entries {
        let face = card.join(format!("{}.png", entry.cid));
        let ours = made.contains_key(&entry.cid) || fetched.contains(&entry.cid);
        let Some(src) = source(entry, folders) else {
            // Its picture is gone: the face made from it goes too.
            if made.remove(&entry.cid).is_some() {
                let _ = std::fs::remove_file(&face);
                done.removed.push(entry.cid);
            }
            continue;
        };
        if made.get(&entry.cid) == Some(&src.label()) && face.exists() { continue }
        // A face the player put there by hand stays, but for the player's own picture.
        if existing.contains(&entry.cid) && !ours && matches!(src, Source::Shipped(_)) { continue }
        let picture = match &src {
            Source::Shipped(code) => Ok(SHIPPED.iter().find(|(c, _)| c == code).map(|(_, b)| b.to_vec()).unwrap_or_default()),
            Source::File(path, _) => match std::fs::metadata(path) {
                Ok(m) if m.len() > MAX_BYTES => Err("the picture is too large".to_string()),
                _ => std::fs::read(path).map_err(|e| e.to_string()),
            },
        };
        match picture.and_then(|bytes| face_from_picture(&bytes)) {
            Ok(rgba) => {
                write_png(&face, FACE, &rgba)?;
                made.insert(entry.cid, src.label());
                fetched.remove(&entry.cid);
                done.made.push(entry.cid);
            }
            Err(why) => done.refused.push((match src { Source::File(p, _) => p, Source::Shipped(c) => PathBuf::from(format!("shipped {c}")) }, why)),
        }
    }
    save_made(&made_file, &made)?;
    if fetched != downloaded(art) { save_ids(&card.join(DOWNLOADED), &fetched)?; }
    Ok(done)
}

/// The player's `pics` folders for an art folder: beside it (the launcher's data folder), then beside the game.
pub fn folders(art: &Path, game: Option<&Path>) -> Vec<PathBuf> {
    let mut out = vec![art.parent().unwrap_or(art).join(FOLDER)];
    if let Some(game) = game { out.push(game.join(FOLDER)) }
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yo3-pics-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A generated picture (never a card's): one colour, so which source made a face is seen in its pixels.
    fn picture(path: &Path, (w, h): (u32, u32), colour: [u8; 3]) {
        let rgba: Vec<u8> = (0..w * h).flat_map(|_| [colour[0], colour[1], colour[2], 255]).collect();
        write_png(path, (w, h), &rgba).unwrap();
    }

    fn colour_of(path: &Path) -> [u8; 3] {
        let (_, _, rgba) = super::super::read_png(path).unwrap();
        [rgba[0], rgba[1], rgba[2]]
    }

    const GEH: Entry = Entry { cid: 12671, anime: 170000170, official: 82103466 };
    const RAIGEKI: Entry = Entry { cid: 4343, anime: 511000001, official: 12580477 };

    #[test]
    fn the_shipped_pictures_are_card_faces_and_each_is_in_the_list() {
        let list = super::super::get::list();
        for (code, bytes) in SHIPPED {
            let (w, h, _) = super::super::decode_png(io::Cursor::new(bytes)).unwrap();
            assert_eq!((w, h), FACE, "{code}");
            assert!(list.iter().any(|e| e.anime == *code || e.official == *code), "{code} is no card of the picture list");
        }
        let files = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/pics")).unwrap().count();
        assert_eq!(files, SHIPPED.len(), "every file of assets/pics is shipped");
    }

    #[test]
    fn a_players_picture_by_either_passcode_becomes_the_face_of_any_size() {
        let dir = scratch("either");
        let (art, pics) = (dir.join("anime-art"), dir.join("pics"));
        std::fs::create_dir_all(&pics).unwrap();
        picture(&pics.join("12580477.png"), (813, 1185), [10, 200, 30]); // EDOPro's high-resolution size, the official passcode
        let done = apply(&art, &[&pics], &[RAIGEKI]).unwrap();
        assert_eq!(done.made, vec![4343]);
        let (w, h, _) = super::super::read_png(&art.join("card/4343.png")).unwrap();
        assert_eq!((w, h), FACE);
        assert_eq!(colour_of(&art.join("card/4343.png")), [10, 200, 30]);
        // The anime passcode wins over the official one.
        picture(&pics.join("511000001.png"), (177, 254), [200, 10, 10]);
        assert_eq!(apply(&art, &[&pics], &[RAIGEKI]).unwrap().made, vec![4343]);
        assert_eq!(colour_of(&art.join("card/4343.png")), [200, 10, 10]);
        // Nothing changed: nothing to do.
        assert!(apply(&art, &[&pics], &[RAIGEKI]).unwrap().made.is_empty());
    }

    #[test]
    fn the_players_picture_wins_over_the_shipped_one_and_the_shipped_one_comes_back_when_it_goes() {
        let dir = scratch("order");
        let (art, pics) = (dir.join("anime-art"), dir.join("pics"));
        std::fs::create_dir_all(&pics).unwrap();
        assert_eq!(apply(&art, &[&pics], &[GEH]).unwrap().made, vec![12671], "the shipped Geh");
        let shipped = std::fs::read(art.join("card/12671.png")).unwrap();
        picture(&pics.join("170000170.png"), (1028, 1530), [1, 2, 3]);
        assert_eq!(apply(&art, &[&pics], &[GEH]).unwrap().made, vec![12671]);
        assert_eq!(colour_of(&art.join("card/12671.png")), [1, 2, 3]);
        std::fs::remove_file(pics.join("170000170.png")).unwrap();
        assert_eq!(apply(&art, &[&pics], &[GEH]).unwrap().made, vec![12671]);
        assert_eq!(std::fs::read(art.join("card/12671.png")).unwrap(), shipped);
    }

    #[test]
    fn a_hand_made_face_is_kept_over_a_shipped_picture_a_downloaded_one_is_not() {
        let dir = scratch("hand");
        let art = dir.join("anime-art");
        std::fs::create_dir_all(art.join("card")).unwrap();
        picture(&art.join("card/12671.png"), FACE, [9, 9, 9]);
        assert!(apply(&art, &[], &[GEH]).unwrap().made.is_empty());
        assert_eq!(colour_of(&art.join("card/12671.png")), [9, 9, 9], "the player's face");
        // The same face listed as downloaded is Desktop's: the shipped picture replaces it, and it is no longer downloaded.
        save_ids(&art.join("card").join(DOWNLOADED), &BTreeSet::from([12671])).unwrap();
        assert_eq!(apply(&art, &[], &[GEH]).unwrap().made, vec![12671]);
        assert!(downloaded(&art).is_empty());
    }

    #[test]
    fn a_removed_picture_takes_its_face_and_a_bad_one_is_refused() {
        let dir = scratch("gone");
        let (art, pics) = (dir.join("anime-art"), dir.join("pics"));
        std::fs::create_dir_all(&pics).unwrap();
        picture(&pics.join("12580477.png"), (177, 254), [5, 5, 5]);
        apply(&art, &[&pics], &[RAIGEKI]).unwrap();
        std::fs::remove_file(pics.join("12580477.png")).unwrap();
        assert_eq!(apply(&art, &[&pics], &[RAIGEKI]).unwrap().removed, vec![4343]);
        assert!(!art.join("card/4343.png").exists());
        // Not a card's shape, not a picture: refused, nothing written.
        picture(&pics.join("12580477.png"), (300, 300), [5, 5, 5]);
        std::fs::write(pics.join("511000001.jpg"), b"not a picture").unwrap();
        let done = apply(&art, &[&pics], &[RAIGEKI]).unwrap();
        assert_eq!(done.refused.len(), 1, "the first match is refused: {:?}", done.refused);
        assert!(!art.join("card/4343.png").exists());
    }
}
