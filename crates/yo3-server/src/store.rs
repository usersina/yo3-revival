//! The server's own state on disk: small JSON files in the data directory (accounts, records), written whole
//! through a temporary file so a crash never leaves half a file, and copied into `backups/` at each start.
use std::fs;
use std::io;
use std::path::Path;

use serde_json::Value as Json;

/// How many start-up backups are kept.
const BACKUPS: usize = 10;

/// The file's JSON, or None when it doesn't exist yet.
pub fn read(path: &Path) -> io::Result<Option<Json>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", path.display()))),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// The file's JSON for a side file that may start empty (`Json::Null` when it doesn't exist yet). A file that won't
/// parse is not thrown away: the next save would overwrite it, and the docs invite hand-editing. It is renamed to
/// `NAME.corrupt-<time>` and said so loudly, and the caller starts empty.
pub fn read_or_keep(path: &Path, what: &str) -> Json {
    match read(path) {
        Ok(saved) => saved.unwrap_or(Json::Null),
        Err(e) => {
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S%.3f");
            let mut kept = path.as_os_str().to_owned();
            kept.push(format!(".corrupt-{stamp}"));
            let kept = std::path::PathBuf::from(kept);
            match fs::rename(path, &kept) {
                Ok(()) => tracing::error!("{what} NOT LOADED: {e}. The file was kept as {} (fix it and put it back); starting empty", kept.display()),
                Err(re) => tracing::error!("{what} NOT LOADED: {e}. It could not be moved aside either ({re}); the next save overwrites it"),
            }
            Json::Null
        }
    }
}

/// Writes the whole file (pretty, so the host can read and edit it).
pub fn write(path: &Path, value: &Json) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value).map_err(io::Error::other)?)?;
    fs::rename(&tmp, path)
}

/// Copies the data directory's JSON files into `backups/<time>/`, keeping the newest few.
pub fn back_up(dir: &Path) -> io::Result<()> {
    // Only files that parse are backed up: a damaged file must not rotate the last good copy out.
    let parses = |p: &std::path::PathBuf| match fs::read(p).map(|b| serde_json::from_slice::<Json>(&b)) {
        Ok(Ok(_)) => true,
        _ => {
            tracing::error!("{} does not parse; it is not backed up", p.display());
            false
        }
    };
    let files: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .filter(parses)
            .collect(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if files.is_empty() {
        return Ok(());
    }
    let backups = dir.join("backups");
    let to = backups.join(chrono::Local::now().format("%Y%m%d-%H%M%S").to_string());
    fs::create_dir_all(&to)?;
    for f in &files {
        if let Some(name) = f.file_name() {
            fs::copy(f, to.join(name))?;
        }
    }
    let mut kept: Vec<_> = fs::read_dir(&backups)?.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    kept.sort();
    for old in kept.iter().rev().skip(BACKUPS) {
        fs::remove_dir_all(old)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("yo3-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_file_that_does_not_parse_is_kept_not_overwritten() {
        let d = dir("corrupt");
        let file = d.join("friends.json");
        fs::write(&file, "{ \"friends\": [[1, 2]").unwrap();
        assert_eq!(read_or_keep(&file, "friends"), Json::Null);
        assert!(!file.exists(), "moved aside");
        let kept: Vec<_> = fs::read_dir(&d).unwrap().filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(kept.len(), 1);
        assert!(kept[0].starts_with("friends.json.corrupt-"), "{kept:?}");
        assert_eq!(fs::read_to_string(d.join(&kept[0])).unwrap(), "{ \"friends\": [[1, 2]");
        // A missing file is simply new.
        assert_eq!(read_or_keep(&d.join("none.json"), "none"), Json::Null);
    }

    #[test]
    fn a_damaged_file_is_not_backed_up() {
        let d = dir("backup");
        write(&d.join("good.json"), &serde_json::json!({ "a": 1 })).unwrap();
        fs::write(d.join("bad.json"), "{{{").unwrap();
        back_up(&d).unwrap();
        let backup = fs::read_dir(d.join("backups")).unwrap().next().unwrap().unwrap().path();
        assert!(backup.join("good.json").is_file());
        assert!(!backup.join("bad.json").exists());
        // With nothing sound to copy no backup directory is made, so none of the good ones is rotated out.
        let d = dir("backup-none");
        fs::write(d.join("bad.json"), "{{{").unwrap();
        back_up(&d).unwrap();
        assert!(!d.join("backups").exists());
    }
}
