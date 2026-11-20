//! Pictures in a zip (issue #60): "Add pictures from a file" takes a zip with the folder's own layout, "Save pictures to a file"
//! writes one. The layout is `card/<Konami id>.png` (200x290) and optionally `mini/<Konami id>.png` (50x72), at the root or inside
//! the zip's single top folder.
//!
//! A zip is untrusted: only names of exactly that shape are looked at (anything else is ignored and counted, a name that tries to
//! leave the folder is refused), every entry is read under a size cap whatever the zip claims, and each picture must be a PNG of
//! exactly the size the folder wants, judged as `prepare` judges the folder's files. A picture the player already has (not one
//! Desktop downloaded) is kept; one Desktop downloaded is replaced.
use super::get::{DOWNLOADED, downloaded, save_ids};
use super::{FACE, MINI, decode_png, id_of};
use std::collections::BTreeSet;
use std::io::{self, Read, Seek, Write};
use std::path::Path;

/// The most one picture may weigh in the zip (a face is about 100 KB).
pub const MAX_ENTRY: u64 = 1 << 20;
/// The most all the pictures read from one zip may weigh.
pub const MAX_TOTAL: u64 = 96 << 20;
/// The most entries a zip may have before it is refused as no picture set.
pub const MAX_ENTRIES: usize = 4000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind { Card, Mini }

/// What an import did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Imported {
    pub faces: u32,
    pub minis: u32,
    /// Pictures the folder already had of the player's own: kept.
    pub kept: u32,
    /// Pictures with a right name that were refused, with why (wrong size, not a PNG, too large).
    pub refused: Vec<(String, String)>,
    /// Entries that are not pictures of this layout: ignored.
    pub ignored: u32,
}

/// `card/4021.png` or `top/mini/4021.png` as (kind, id); `Err` for a name that leaves the folder, `Ok(None)` for another name.
fn target(name: &str, single_top: bool) -> Result<Option<(Kind, u32)>, ()> {
    if name.starts_with('/') || name.contains('\\') || name.contains(':') || name.split('/').any(|p| p == "..") { return Err(()) }
    let parts: Vec<&str> = name.split('/').collect();
    let (kind, file) = match parts.as_slice() {
        [kind, file] => (*kind, *file),
        [_, kind, file] if single_top => (*kind, *file),
        _ => return Ok(None),
    };
    let kind = match kind { "card" => Kind::Card, "mini" => Kind::Mini, _ => return Ok(None) };
    Ok(id_of(Path::new(file)).filter(|_| file.len() <= 9).map(|id| (kind, id)))
}

fn mark(dir: &Path, which: &str) -> std::path::PathBuf { dir.join(which).join(if which == "card" { DOWNLOADED } else { ".derived" }) }

fn ids_of(file: &Path) -> BTreeSet<u32> {
    std::fs::read_to_string(file).unwrap_or_default().lines().filter_map(|l| l.trim().parse().ok()).collect()
}

fn put(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("png.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Adds the pictures of the zip `source` to the folder `dir`.
pub fn import(dir: &Path, source: &Path) -> Result<Imported, String> {
    let file = std::fs::File::open(source).map_err(|e| e.to_string())?;
    import_from(dir, file)
}

pub fn import_from(dir: &Path, source: impl Read + Seek) -> Result<Imported, String> {
    let mut zip = zip::ZipArchive::new(source).map_err(|_| "That file is not a zip file.".to_string())?;
    if zip.len() > MAX_ENTRIES { return Err("That zip file holds too many files to be a set of pictures.".into()) }
    let tops: BTreeSet<String> = zip.file_names().filter_map(Result::ok).map(|n| n.split('/').next().unwrap_or("").to_string()).collect();
    let single_top = tops.len() == 1 && !tops.contains("card") && !tops.contains("mini");
    for kind in ["card", "mini"] { std::fs::create_dir_all(dir.join(kind)).map_err(|e| e.to_string())? }
    let (mut ours, mut derived) = (downloaded(dir), ids_of(&mark(dir, "mini")));
    let (mut out, mut total, mut seen) = (Imported::default(), 0u64, BTreeSet::new());
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() { continue }
        let Ok(name) = entry.name().map(|n| n.to_string()) else { out.ignored += 1; continue };
        let (kind, id) = match target(&name, single_top) {
            Err(()) => { out.refused.push((name, "its name leaves the folder".into())); continue }
            Ok(None) => { out.ignored += 1; continue }
            Ok(Some(t)) => t,
        };
        if !seen.insert((kind as u8, id)) { out.ignored += 1; continue }
        if entry.size() > MAX_ENTRY { out.refused.push((name, "it is too large".into())); continue }
        total += entry.size();
        if total > MAX_TOTAL { return Err("That zip file holds more than a set of pictures can.".into()) }
        // The size the zip claims is not trusted: read at most one byte more than allowed.
        let mut bytes = Vec::new();
        entry.by_ref().take(MAX_ENTRY + 1).read_to_end(&mut bytes).map_err(|_| "A picture in that zip file could not be read.".to_string())?;
        if bytes.len() as u64 > MAX_ENTRY { out.refused.push((name, "it is too large".into())); continue }
        let want = if kind == Kind::Card { FACE } else { MINI };
        match decode_png(io::Cursor::new(&bytes)) {
            Ok((w, h, _)) if (w, h) == want => {}
            Ok((w, h, _)) => { out.refused.push((name, format!("it is {w}x{h}, not {}x{}", want.0, want.1))); continue }
            Err(_) => { out.refused.push((name, "it is not a PNG picture".into())); continue }
        }
        let (folder, ours_or_made) = if kind == Kind::Card { ("card", &mut ours) } else { ("mini", &mut derived) };
        let path = dir.join(folder).join(format!("{id}.png"));
        // A picture the folder has that is not marked as made by us is the player's own.
        if path.exists() && !ours_or_made.contains(&id) { out.kept += 1; continue }
        put(&path, &bytes).map_err(|e| e.to_string())?;
        ours_or_made.remove(&id);
        if kind == Kind::Card {
            out.faces += 1;
            // A thumbnail made from the face before is made again from the new one.
            if derived.remove(&id) { let _ = std::fs::remove_file(dir.join("mini").join(format!("{id}.png"))); }
        } else { out.minis += 1 }
    }
    save_ids(&mark(dir, "card"), &ours).map_err(|e| e.to_string())?;
    save_ids(&mark(dir, "mini"), &derived).map_err(|e| e.to_string())?;
    super::prepare(dir).map_err(|e| e.to_string())?;
    Ok(out)
}

/// What an export wrote.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Exported { pub faces: u32, pub minis: u32 }

/// Writes the folder's faces and thumbnails to the zip `target` (written whole, then put in place).
pub fn export(dir: &Path, target: &Path) -> Result<Exported, String> {
    let tmp = target.with_extension("zip.tmp");
    let result = (|| {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&tmp).map_err(|e| e.to_string())?);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let mut out = Exported::default();
        let mut faces: Vec<u32> = std::fs::read_dir(dir.join("card")).into_iter().flatten().filter_map(Result::ok).filter_map(|e| id_of(&e.path())).collect();
        faces.sort_unstable();
        for (folder, want) in [("card", FACE), ("mini", MINI)] {
            for id in &faces {
                let path = dir.join(folder).join(format!("{id}.png"));
                let Ok(bytes) = std::fs::read(&path) else { continue };
                if !matches!(decode_png(io::Cursor::new(&bytes)), Ok((w, h, _)) if (w, h) == want) { continue }
                zip.start_file(format!("{folder}/{id}.png"), options).map_err(|e| e.to_string())?;
                zip.write_all(&bytes).map_err(|e| e.to_string())?;
                if folder == "card" { out.faces += 1 } else { out.minis += 1 }
            }
        }
        zip.finish().map_err(|e| e.to_string())?;
        if out.faces == 0 { return Err("There are no pictures to save yet.".to_string()) }
        std::fs::rename(&tmp, target).map_err(|e| e.to_string())?;
        Ok(out)
    })();
    if result.is_err() { let _ = std::fs::remove_file(&tmp); }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::write_png;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yo3-art-pack-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }
    /// A generated PNG (never a card) as bytes.
    fn png(size: (u32, u32), shade: u8) -> Vec<u8> {
        let dir = scratch("png");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("x.png");
        write_png(&path, size, &vec![shade; (size.0 * size.1 * 4) as usize]).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        bytes
    }
    fn zip_of(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }
    fn import_bytes(dir: &Path, bytes: Vec<u8>) -> Result<Imported, String> { import_from(dir, io::Cursor::new(bytes)) }

    #[test]
    fn the_folders_layout_is_imported_at_the_root_or_in_a_single_top_folder() {
        let (face, mini) = (png(FACE, 10), png(MINI, 20));
        for prefix in ["", "pictures/"] {
            let dir = scratch("layout");
            let zip = zip_of(&[(&format!("{prefix}card/4021.png"), face.clone()), (&format!("{prefix}mini/4021.png"), mini.clone()), (&format!("{prefix}card/4008.png"), face.clone())]);
            let done = import_bytes(&dir, zip).unwrap();
            assert_eq!((done.faces, done.minis, done.refused.len()), (2, 1, 0), "{prefix}");
            assert_eq!(std::fs::read(dir.join("mini/4021.png")).unwrap(), mini, "a thumbnail in the zip is used as it is");
            assert!(dir.join("mini/4008.png").exists(), "the other face's thumbnail is made");
            assert_eq!(downloaded(&dir), BTreeSet::new(), "imported pictures are the player's own");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn what_is_not_a_picture_of_the_layout_is_ignored_and_what_is_wrong_is_refused() {
        let dir = scratch("refuse");
        let face = png(FACE, 10);
        let zip = zip_of(&[
            ("card/1.png", face.clone()),
            ("card/2.png", png((100, 100), 1)),
            ("card/3.png", b"not a png".to_vec()),
            ("mini/4.png", png(FACE, 1)),
            ("card/5.png", vec![0; (MAX_ENTRY + 10) as usize]),
            ("../card/6.png", face.clone()),
            ("/card/7.png", face.clone()),
            ("card/..\\8.png", face.clone()),
            ("card/sub/9.png", face.clone()),
            ("card/4021_1.png", face.clone()),
            ("card/abc.png", face.clone()),
            ("card/123456.png", face.clone()),
            ("data/card/10.png", face.clone()),
            ("readme.txt", b"hello".to_vec()),
            ("card/11.PNG.exe", face.clone()),
        ]);
        let done = import_bytes(&dir, zip).unwrap();
        assert_eq!((done.faces, done.minis), (1, 0));
        let refused: Vec<&str> = done.refused.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(refused, ["card/2.png", "card/3.png", "mini/4.png", "card/5.png", "../card/6.png", "/card/7.png", "card/..\\8.png"]);
        assert!(done.refused[0].1.contains("100x100") && done.refused[3].1.contains("too large") && done.refused[4].1.contains("leaves"));
        assert_eq!(done.ignored, 7);
        let mut names: Vec<String> = std::fs::read_dir(dir.join("card")).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, vec![".downloaded", "1.png"], "nothing else was written, nothing outside the folder");
        assert!(!dir.parent().unwrap().join("card").join("6.png").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_zip_of_many_files_or_no_zip_is_refused_whole() {
        let dir = scratch("many");
        assert!(import_bytes(&dir, b"PK nope".to_vec()).unwrap_err().contains("not a zip"));
        let many: Vec<(String, Vec<u8>)> = (0..MAX_ENTRIES + 1).map(|i| (format!("x/{i}.txt"), vec![])).collect();
        let zip = zip_of(&many.iter().map(|(n, b)| (n.as_str(), b.clone())).collect::<Vec<_>>());
        assert!(import_bytes(&dir, zip).unwrap_err().contains("too many"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_players_own_pictures_are_kept_and_the_downloaded_ones_are_replaced() {
        let dir = scratch("keep");
        std::fs::create_dir_all(dir.join("card")).unwrap();
        let own = png(FACE, 77);
        std::fs::write(dir.join("card/1.png"), &own).unwrap();
        std::fs::write(dir.join("card/2.png"), png(FACE, 5)).unwrap();
        std::fs::write(dir.join("card").join(DOWNLOADED), "2\n").unwrap();
        let new = png(FACE, 200);
        let done = import_bytes(&dir, zip_of(&[("card/1.png", new.clone()), ("card/2.png", new.clone()), ("card/3.png", new.clone())])).unwrap();
        assert_eq!((done.faces, done.kept), (2, 1));
        assert_eq!(std::fs::read(dir.join("card/1.png")).unwrap(), own, "the player's own picture stays");
        assert_eq!(std::fs::read(dir.join("card/2.png")).unwrap(), new, "a picture Desktop downloaded gives way to the player's file");
        assert_eq!(downloaded(&dir), BTreeSet::new());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saved_pictures_come_back_the_same_through_a_zip() {
        let (from, to, file) = (scratch("from"), scratch("to"), scratch("file").with_extension("zip"));
        std::fs::create_dir_all(from.join("card")).unwrap();
        std::fs::create_dir_all(from.join("mini")).unwrap();
        for (id, shade) in [(4021u32, 30u8), (4008, 60)] { std::fs::write(from.join(format!("card/{id}.png")), png(FACE, shade)).unwrap() }
        std::fs::write(from.join("mini/4021.png"), png(MINI, 99)).unwrap();
        std::fs::write(from.join("card/9.png"), png((10, 10), 1)).unwrap();
        std::fs::write(from.join("card/notes.txt"), "x").unwrap();
        assert_eq!(export(&from, &file).unwrap(), Exported { faces: 2, minis: 1 });
        let done = import(&to, &file).unwrap();
        assert_eq!((done.faces, done.minis, done.refused.len()), (2, 1, 0));
        for name in ["card/4021.png", "card/4008.png", "mini/4021.png"] {
            assert_eq!(std::fs::read(from.join(name)).unwrap(), std::fs::read(to.join(name)).unwrap(), "{name}");
        }
        assert!(to.join("mini/4008.png").exists());
        assert!(export(&scratch("empty"), &file.with_extension("other.zip")).unwrap_err().contains("no pictures"));
        assert!(!file.with_extension("other.zip").exists() && !file.with_extension("zip.tmp").exists());
        for d in [from, to] { let _ = std::fs::remove_dir_all(&d); }
        let _ = std::fs::remove_file(&file);
    }
}
