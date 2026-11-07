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
    let files: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect(),
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
