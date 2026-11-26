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
        // `data/bin#/CARD_Prop.bin` -> `CARD_Prop.bin`, `data/regulation/20.y3r` (a banlist with the anime-only cards
        // Forbidden) -> `regulation/20.y3r`: the DLL looks a file up by its last part (tools/mod/added-cards-logic.h).
        // The Anime Collection pack's file and the UI text tables likewise: `pack/300.y3a`, `text/txttbl_E.bin`.
        // A custom NPC's body keeps its own folder: `data/model/np025/np0050.png` -> `model/np025/np0050.png`.
        // Its portrait and faces likewise: `npc/npcsel025.png`, `face/np025f0.png`.
        let (sub, leaf) = match name.strip_prefix("data/model/").and_then(|rest| rest.split_once('/')) {
            Some((folder, leaf)) => (folder, leaf),
            None => ("", name.rsplit('/').next().unwrap_or(name)),
        };
        let at = match ["regulation", "pack", "text", "model", "npc", "face"].into_iter().find(|kind| name.starts_with(&format!("data/{kind}/"))) {
            Some(kind) => staging.join(kind),
            None => staging.clone(),
        };
        let at = if sub.is_empty() { at } else { at.join(sub) };
        std::fs::create_dir_all(&at)?;
        std::fs::write(at.join(leaf), bytes)?;
    }
    // The pack's picture (data/img/pack/, 145x290), with its file.
    if files.iter().any(|(name, _)| name.starts_with("data/pack/")) {
        std::fs::create_dir_all(staging.join("img/pack"))?;
        std::fs::write(staging.join("img/pack").join(yo3_data::added::ANIME_PACK_IMAGE), ANIME_PACK_PICTURE)?;
    }
    std::fs::write(staging.join("complete"), format!("{} file(s)\n", files.len()))?;
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::rename(&staging, &folder)?;
    clean(base, &folder);
    copy_pictures(&folder, added, pictures);
    Ok(Prepared { folder: Some(folder), written: files.len() })
}

/// The Anime Collection pack's picture (the owner's art, 145x290).
pub const ANIME_PACK_PICTURE: &[u8] = include_bytes!("../assets/packs/YO3_ANIME_1.png");

/// The face an added card shows while it has no picture of its own (`tools/engine/no-picture.py` makes it, 200x290).
pub const NO_PICTURE: &[u8] = include_bytes!("../assets/no-picture.png");

/// Each added card's picture from the picture folder (`card/<id>.png`, `mini/<id>.png`: Desktop's Get pictures fetches
/// them with the anime ones, by the card's official passcode) into `folder`, at every start, so a picture fetched since the
/// last one shows. A card without one shows [`NO_PICTURE`] (and its thumbnail), never an empty frame.
fn copy_pictures(folder: &Path, added: &yo3_data::added::Added, pictures: Option<&Path>) {
    for card in &added.cards {
        let have = |kind: &str| pictures.map(|p| p.join(kind).join(format!("{}.png", card.cid))).filter(|p| p.is_file());
        let (face, mini) = (have("card"), have("mini"));
        for (kind, from) in [("card", &face), ("mini", if face.is_some() { &mini } else { &None })] {
            let to = folder.join(kind).join(format!("{}.png", card.cid));
            if std::fs::create_dir_all(folder.join(kind)).is_err() { continue }
            match (from, kind) {
                (Some(from), _) => { let _ = std::fs::copy(from, &to); }
                (None, "card") => { let _ = std::fs::write(&to, NO_PICTURE); }
                // A downloaded face without its thumbnail: the game's own folder thumbnail is made from the face it shows.
                (None, _) => { let _ = write_mini(face.as_deref(), &to); }
            }
        }
    }
}

/// The 50x72 thumbnail of `face` (or of [`NO_PICTURE`]) written to `to`.
fn write_mini(face: Option<&Path>, to: &Path) -> Result<(), String> {
    let (w, h, rgba) = match face {
        Some(face) => crate::art::decode_png(std::io::BufReader::new(std::fs::File::open(face).map_err(|e| e.to_string())?))?,
        None => crate::art::decode_png(std::io::Cursor::new(NO_PICTURE))?,
    };
    if rgba.is_empty() { return Err("the face is too large".into()) }
    crate::art::write_png(to, crate::art::MINI, &crate::art::downscale(&rgba, (w, h), crate::art::MINI)).map_err(|e| e.to_string())
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

    fn quasar() -> yo3_data::added::Added {
        yo3_data::added::Added::parse(include_str!("../../../engine/added-cards.json")).unwrap()
    }

    #[test]
    fn a_card_without_a_picture_shows_the_shipped_one_until_its_own_arrives() {
        let dir = std::env::temp_dir().join(format!("yo3-added-pics-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (folder, pictures) = (dir.join("list"), dir.join("art"));
        let added = quasar();
        let cid = added.cards[0].cid;
        copy_pictures(&folder, &added, None);
        assert_eq!(std::fs::read(folder.join(format!("card/{cid}.png"))).unwrap(), NO_PICTURE);
        let (w, h, _) = crate::art::decode_png(std::io::BufReader::new(std::fs::File::open(folder.join(format!("mini/{cid}.png"))).unwrap())).unwrap();
        assert_eq!((w, h), crate::art::MINI);
        let (fw, fh, _) = crate::art::decode_png(std::io::Cursor::new(NO_PICTURE)).unwrap();
        assert_eq!((fw, fh), crate::art::FACE, "the shipped face is a card face");
        // Its own face arrives (Get pictures): it replaces the stand-in at the next start, thumbnail included.
        std::fs::create_dir_all(pictures.join("card")).unwrap();
        let own = vec![200u8; (crate::art::FACE.0 * crate::art::FACE.1 * 4) as usize];
        crate::art::write_png(&pictures.join(format!("card/{cid}.png")), crate::art::FACE, &own).unwrap();
        copy_pictures(&folder, &added, Some(&pictures));
        assert_ne!(std::fs::read(folder.join(format!("card/{cid}.png"))).unwrap(), NO_PICTURE);
        let (_, _, mini) = crate::art::decode_png(std::io::BufReader::new(std::fs::File::open(folder.join(format!("mini/{cid}.png"))).unwrap())).unwrap();
        assert_eq!(mini[0], 200, "the thumbnail is made from its own face");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_anime_collection_pack_is_written_with_its_picture_and_texts() {
        // The launcher's folder holds what the DLL serves for pack 300: `pack/300.y3a`, `img/pack/YO3_ANIME_1.png` (145x290)
        // and the UI text tables. Needs the game's files.
        let game = std::env::var_os("YO3_GAME_DIR").map(PathBuf::from).unwrap_or_else(yo3_data::default_game_dir);
        if !game.join("package/data002.ypk").is_file() { return }
        let dir = std::env::temp_dir().join(format!("yo3-added-pack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let done = prepare(&dir, &game, &quasar(), None).unwrap();
        let folder = done.folder.unwrap();
        assert!(folder.join("pack/300.y3a").is_file() && folder.join("text/txttbl_E.bin").is_file());
        let (w, h, _) = crate::art::decode_png(std::io::Cursor::new(std::fs::read(folder.join("img/pack/YO3_ANIME_1.png")).unwrap())).unwrap();
        assert_eq!((w, h), (145, 290), "the size of the game's pack pictures");
        let _ = std::fs::remove_dir_all(&dir);
    }

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
