//! The anime artwork folder (issue #60, `docs/anime-cards.md`): pictures a player or host provides for the cards the card
//! profile shows as their anime versions. Nothing here ships or fetches an image. The launcher creates the folder, hands
//! its path to the game as `YO3_ANIME_ART` (the connection DLL reads it) and keeps the thumbnails in step with the faces.
//!
//! ```text
//! anime-art/card/<Konami id>.png   the card face, 200x290: the game's data/card/<id>_<art>.png, for every artwork of the card
//! anime-art/mini/<Konami id>.png   the thumbnail, 50x72 (the game's data/mini/): optional, made from the face when missing
//! ```
//!
//! A card without a file keeps its own artwork. A thumbnail the launcher made is listed in `mini/.derived` and made again when
//! its face changes; one the player provided is never touched.
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

/// The game's environment variable the DLL reads.
pub const ENV: &str = "YO3_ANIME_ART";
/// A card face and a thumbnail, as the game's own files.
pub const FACE: (u32, u32) = (200, 290);
pub const MINI: (u32, u32) = (50, 72);
/// The folder's name under the launcher's data folder, and beside the game as the fallback.
pub const FOLDER: &str = "anime-art";
const DERIVED: &str = ".derived";

/// What a preparation did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Prepared {
    /// Faces found (Konami ids).
    pub faces: BTreeSet<u32>,
    /// Thumbnails made or made again.
    pub made: Vec<u32>,
    /// Files skipped, with why (not a PNG, not 200x290 ...): the game would show the original artwork.
    pub skipped: Vec<(PathBuf, String)>,
}

/// The Konami id a file is named for: `4021.png`.
fn id_of(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_suffix(".png").or_else(|| name.strip_suffix(".PNG"))?;
    (!stem.is_empty() && stem.len() <= 5 && stem.bytes().all(|b| b.is_ascii_digit())).then(|| stem.parse().ok()).flatten()
}

fn read_png(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    // A face is 200x290: anything much larger is refused before it is decoded.
    if reader.info().width > 2048 || reader.info().height > 2048 { return Ok((reader.info().width, reader.info().height, vec![])) }
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
    Ok((info.width, info.height, rgba))
}

/// `rgba` (w x h) scaled to `to` by averaging the source pixels each target pixel covers (a box filter, alpha-weighted).
pub fn downscale(rgba: &[u8], (w, h): (u32, u32), to: (u32, u32)) -> Vec<u8> {
    let mut out = Vec::with_capacity((to.0 * to.1 * 4) as usize);
    for y in 0..to.1 {
        let (y0, y1) = (y * h / to.1, ((y + 1) * h).div_ceil(to.1).min(h));
        for x in 0..to.0 {
            let (x0, x1) = (x * w / to.0, ((x + 1) * w).div_ceil(to.0).min(w));
            let (mut sum, mut alpha, mut n) = ([0u64; 3], 0u64, 0u64);
            for sy in y0..y1.max(y0 + 1) {
                for sx in x0..x1.max(x0 + 1) {
                    let p = &rgba[((sy * w + sx) * 4) as usize..][..4];
                    for c in 0..3 { sum[c] += u64::from(p[c]) * u64::from(p[3]); }
                    alpha += u64::from(p[3]);
                    n += 1;
                }
            }
            out.extend(sum.map(|s| s.checked_div(alpha).unwrap_or(0) as u8));
            out.push((alpha / n.max(1)) as u8);
        }
    }
    out
}

fn write_png(path: &Path, (w, h): (u32, u32), rgba: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("png.tmp");
    {
        let mut encoder = png::Encoder::new(io::BufWriter::new(std::fs::File::create(&tmp)?), w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().and_then(|mut writer| writer.write_image_data(rgba)).map_err(io::Error::other)?;
    }
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
fn size_of(path: &Path) -> Result<(u32, u32), String> {
    read_png(path).map(|(w, h, _)| (w, h))
}

/// Creates the folder (`card/` and `mini/` in it) and makes the thumbnails the faces lack: each `card/<id>.png` without a
/// `mini/<id>.png` gets one, 50x72, and a thumbnail made earlier is made again when its face is newer. A face that is not a
/// 200x290 PNG is skipped and reported, so the game keeps the original artwork rather than a picture drawn at the wrong
/// size. Never writes outside `dir`, never touches a face.
pub fn prepare(dir: &Path) -> io::Result<Prepared> {
    let (card, mini) = (dir.join("card"), dir.join("mini"));
    std::fs::create_dir_all(&card)?;
    std::fs::create_dir_all(&mini)?;
    let derived_file = mini.join(DERIVED);
    let mut derived: BTreeSet<u32> = std::fs::read_to_string(&derived_file).unwrap_or_default().lines().filter_map(|l| l.trim().parse().ok()).collect();
    let mut done = Prepared::default();
    let mut faces: Vec<(u32, PathBuf)> = std::fs::read_dir(&card)?.filter_map(Result::ok).filter_map(|e| Some((id_of(&e.path())?, e.path()))).collect();
    faces.sort();
    for (id, face) in faces {
        let (w, h, rgba) = match read_png(&face) {
            Ok(png) if (png.0, png.1) == FACE => png,
            Ok((w, h, _)) => { done.skipped.push((face, format!("it is {w}x{h}, the game's card faces are {}x{}", FACE.0, FACE.1))); continue }
            Err(e) => { done.skipped.push((face, format!("not a PNG the launcher can read ({e})"))); continue }
        };
        done.faces.insert(id);
        let thumb = mini.join(format!("{id}.png"));
        let stale = derived.contains(&id) && match (std::fs::metadata(&thumb).and_then(|m| m.modified()), std::fs::metadata(&face).and_then(|m| m.modified())) {
            (Ok(t), Ok(f)) => f > t,
            _ => true,
        };
        if thumb.exists() && !(derived.contains(&id) && stale) { continue }
        write_png(&thumb, MINI, &downscale(&rgba, (w, h), MINI))?;
        derived.insert(id);
        done.made.push(id);
    }
    if !done.made.is_empty() {
        std::fs::write(&derived_file, derived.iter().map(|i| format!("{i}\n")).collect::<String>())?;
    }
    // A thumbnail of a face that is gone, which the launcher made, goes too: the game would show a picture of nothing.
    for id in derived.clone() {
        if !card.join(format!("{id}.png")).exists() {
            let _ = std::fs::remove_file(mini.join(format!("{id}.png")));
            derived.remove(&id);
            let _ = std::fs::write(&derived_file, derived.iter().map(|i| format!("{i}\n")).collect::<String>());
        }
    }
    Ok(done)
}

/// Which of `wanted` (the Konami ids a card profile shows as anime cards) have a usable face and which do not.
pub fn coverage(prepared: &Prepared, wanted: &BTreeSet<u32>) -> (Vec<u32>, Vec<u32>) {
    wanted.iter().partition(|id| prepared.faces.contains(id))
}

/// The folder for a launcher whose data lives in `root`: `root/anime-art`.
pub fn folder_in(root: &Path) -> PathBuf {
    root.join(FOLDER)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A generated placeholder image (never a card): a gradient with a red 4x4 corner.
    fn placeholder(path: &Path, (w, h): (u32, u32)) {
        let rgba: Vec<u8> = (0..w * h).flat_map(|i| { let (x, y) = (i % w, i / w); if x < 4 && y < 4 { [255, 0, 0, 255] } else { [(x % 256) as u8, (y % 256) as u8, 128, 255] } }).collect();
        write_png(path, (w, h), &rgba).unwrap();
    }
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yo3-art-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_face_without_a_thumbnail_gets_one_and_a_player_s_own_is_kept() {
        let dir = scratch("mini");
        let faces = dir.join("card");
        std::fs::create_dir_all(&faces).unwrap();
        placeholder(&faces.join("4021.png"), FACE);
        placeholder(&faces.join("4008.png"), FACE);
        std::fs::create_dir_all(dir.join("mini")).unwrap();
        placeholder(&dir.join("mini/4008.png"), MINI);
        let own = std::fs::read(dir.join("mini/4008.png")).unwrap();
        let done = prepare(&dir).unwrap();
        assert_eq!((done.faces.iter().copied().collect::<Vec<_>>(), done.made.clone()), (vec![4008, 4021], vec![4021]));
        assert_eq!(size_of(&dir.join("mini/4021.png")).unwrap(), MINI);
        assert_eq!(std::fs::read(dir.join("mini/4008.png")).unwrap(), own, "the player's thumbnail is untouched");
        // A second preparation has nothing to do.
        assert!(prepare(&dir).unwrap().made.is_empty());
        // A new face makes its thumbnail again; the corner of the downscale is still red.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        placeholder(&faces.join("4021.png"), FACE);
        assert_eq!(prepare(&dir).unwrap().made, vec![4021]);
        let (w, _, rgba) = read_png(&dir.join("mini/4021.png")).unwrap();
        assert!(rgba[0] > 100 && rgba[1] < 150 && w == MINI.0, "the corner stays reddish: {:?}", &rgba[..4]);
        // The face goes: so does the thumbnail the launcher made, not the player's.
        std::fs::remove_file(faces.join("4021.png")).unwrap();
        std::fs::remove_file(faces.join("4008.png")).unwrap();
        prepare(&dir).unwrap();
        assert!(!dir.join("mini/4021.png").exists() && dir.join("mini/4008.png").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_picture_of_the_wrong_size_or_kind_is_skipped_and_named() {
        let dir = scratch("wrong");
        std::fs::create_dir_all(dir.join("card")).unwrap();
        placeholder(&dir.join("card/1.png"), (100, 100));
        std::fs::write(dir.join("card/2.png"), b"not a png").unwrap();
        std::fs::write(dir.join("card/readme.txt"), b"x").unwrap();
        let done = prepare(&dir).unwrap();
        assert!(done.faces.is_empty() && done.made.is_empty());
        let why: Vec<&str> = done.skipped.iter().map(|(_, w)| w.as_str()).collect();
        assert!(why[0].contains("100x100") && why[1].contains("not a PNG"), "{why:?}");
        assert_eq!(coverage(&done, &BTreeSet::from([1, 2, 3])), (vec![], vec![1, 2, 3]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_downscale_averages_and_keeps_the_size() {
        let white = vec![255u8; 200 * 290 * 4];
        let small = downscale(&white, FACE, MINI);
        assert_eq!(small.len(), (MINI.0 * MINI.1 * 4) as usize);
        assert!(small.iter().all(|&b| b == 255));
        assert_eq!(id_of(Path::new("x/4021.png")), Some(4021));
        assert_eq!(id_of(Path::new("x/4021_1.png")), None);
        assert_eq!(folder_in(Path::new("/d")), PathBuf::from("/d/anime-art"));
    }
}
