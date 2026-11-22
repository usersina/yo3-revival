//! The cards the game never had (#75): before the game is released, the launcher asks the server for its added cards
//! (`/web/added-cards`, through its own relay), builds the game's card files with them from the player's own game
//! (`yo3_data::added::extend_files`) and tells the connection DLL where they are. The DLL hands the game those files in place
//! of its own when it loads its card data (`tools/mod/added-cards.h`).
//!
//! The DLL learns the folder from a small file named before the game starts ([`ENV`]): the launcher writes the folder
//! into it once it has signed in (the game is still suspended then), or leaves it empty when the server adds nothing.
//! Each list's files are kept in `<base>/<hash>/` and reused by the next start; older lists' folders are removed.
use std::path::{Path, PathBuf};

/// The variable naming the pointer file for the DLL (a Windows path).
pub const ENV: &str = "YO3_ADDED_CARDS";

/// The added cards as the game will need them, asked of the launcher's own relay (`port`, under the launch cookie).
pub async fn fetch(port: u16, cookie: &str) -> anyhow::Result<yo3_data::added::Added> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::time::timeout(std::time::Duration::from_secs(5), tokio::net::TcpStream::connect(("127.0.0.1", port))).await??;
    stream.write_all(format!("GET /session/{cookie}/web/added-cards HTTP/1.0\r\nHost: localhost\r\n\r\n").as_bytes()).await?;
    let mut reply = vec![];
    tokio::time::timeout(std::time::Duration::from_secs(10), stream.take(8 << 20).read_to_end(&mut reply)).await??;
    let text = String::from_utf8_lossy(&reply).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").ok_or_else(|| anyhow::anyhow!("no answer"))?;
    // A server from before #75 has no such page: it adds nothing.
    if head.starts_with("HTTP/1.1 404") || head.starts_with("HTTP/1.0 404") { return Ok(Default::default()) }
    anyhow::ensure!(head.starts_with("HTTP/1.1 200") || head.starts_with("HTTP/1.0 200"), "the server answered: {}", head.lines().next().unwrap_or(""));
    yo3_data::added::Added::parse(body).map_err(anyhow::Error::msg)
}

/// Where the extended files go for the launcher whose profile is in `profile_dir`.
pub fn base_for(profile_dir: &Path) -> PathBuf {
    profile_dir.join("added-cards")
}

/// What [`prepare`] did.
#[derive(Debug, PartialEq)]
pub struct Prepared {
    /// The folder of files the DLL serves (`<base>/<hash>`), none when nothing is added.
    pub folder: Option<PathBuf>,
    /// Files written now (0 when the folder was there from an earlier start).
    pub written: usize,
}

/// Builds `added`'s card files from the game in `game_dir` into `base/<hash>/` (kept when complete already) and removes
/// other lists' folders. Nothing to do when nothing is added.
pub fn prepare(base: &Path, game_dir: &Path, added: &yo3_data::added::Added, pictures: Option<&Path>) -> anyhow::Result<Prepared> {
    if added.is_empty() {
        return Ok(Prepared { folder: None, written: 0 });
    }
    let folder = base.join(added.hash());
    let done = folder.join("complete");
    if done.is_file() {
        clean(base, &folder);
        copy_pictures(&folder, added, pictures);
        return Ok(Prepared { folder: Some(folder), written: 0 });
    }
    let ypk = yo3_data::Ypk::open(game_dir.join("package/data002.ypk"))?;
    let files = yo3_data::added::extend_files(|name| ypk.read(name).ok().flatten(), added).map_err(anyhow::Error::msg)?;
    let staging = base.join(format!("{}.part", added.hash()));
    let _ = std::fs::remove_dir_all(&staging);
    for (name, bytes) in &files {
        // `data/bin#/CARD_Prop.bin` -> `CARD_Prop.bin`: the DLL looks a file up by its last part.
        let leaf = name.rsplit('/').next().unwrap_or(name);
        std::fs::create_dir_all(&staging)?;
        std::fs::write(staging.join(leaf), bytes)?;
    }
    std::fs::write(staging.join("complete"), format!("{} file(s)\n", files.len()))?;
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::rename(&staging, &folder)?;
    clean(base, &folder);
    copy_pictures(&folder, added, pictures);
    Ok(Prepared { folder: Some(folder), written: files.len() })
}

/// Each added card's picture from the picture folder (`card/<id>.png`, `mini/<id>.png`: Desktop's Get pictures fetches
/// them with the anime ones, by the card's official passcode) into `folder`, at every start, so a picture fetched since the
/// last one shows. A card without one shows no picture in the game.
fn copy_pictures(folder: &Path, added: &yo3_data::added::Added, pictures: Option<&Path>) {
    let Some(pictures) = pictures else { return };
    for card in &added.cards {
        for kind in ["card", "mini"] {
            let from = pictures.join(kind).join(format!("{}.png", card.cid));
            if from.is_file() && std::fs::create_dir_all(folder.join(kind)).is_ok() {
                let _ = std::fs::copy(&from, folder.join(kind).join(format!("{}.png", card.cid)));
            }
        }
    }
}

/// Removes every list's folder in `base` but `keep`.
fn clean(base: &Path, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(base) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path != keep && path.is_dir() { let _ = std::fs::remove_dir_all(path); }
    }
}

/// The pointer file the DLL reads (`base/pointer-<id>.txt`, one per start): written empty before the game starts, then
/// with the folder (a Windows path) by [`point`].
pub fn pointer_for(base: &Path, id: &str) -> PathBuf {
    base.join(format!("pointer-{id}.txt"))
}

/// Writes the folder the DLL serves (its Windows path, or nothing) into the pointer file.
pub fn point(pointer: &Path, folder: Option<&str>) -> std::io::Result<()> {
    if let Some(parent) = pointer.parent() { std::fs::create_dir_all(parent)? }
    std::fs::write(pointer, folder.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_added_prepares_nothing_and_a_pointer_can_be_empty() {
        let dir = std::env::temp_dir().join(format!("yo3-added-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(prepare(&dir, Path::new("/nonexistent"), &Default::default(), None).unwrap(), Prepared { folder: None, written: 0 });
        let pointer = pointer_for(&dir, "a1");
        point(&pointer, None).unwrap();
        assert_eq!(std::fs::read_to_string(&pointer).unwrap(), "");
        point(&pointer, Some(r"C:\x\added-cards\abc")).unwrap();
        assert_eq!(std::fs::read_to_string(&pointer).unwrap(), r"C:\x\added-cards\abc");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
