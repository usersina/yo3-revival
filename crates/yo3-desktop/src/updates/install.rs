//! Install complete bundles beside previous versions. Original files and user data are never replaced.
use std::path::{Path, PathBuf, Component};
use std::io::{Read, Write};
use anyhow::{Context, ensure};
use serde::{Serialize, Deserialize};
use super::{Release, MAX_DOWNLOAD};
use yo3_tunnel::diagnostics::{private_create, private_dir};
const MAX_UNPACK: u64 = 1024 * 1024 * 1024;
const MAX_FILES: usize = 50_000;
const RECEIPT: &str = ".yo3-update.json";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Installed {
    pub tag: String, pub directory: PathBuf, pub platform: String, pub manifest_sha256: String,
    pub desktop_sha256: String, pub archive_sha256: String,
}
impl Installed {
    pub fn executable(&self) -> PathBuf { self.directory.join(if self.platform == "windows-x86_64" { "yo3-revival.exe" } else { "yo3-revival" }) }
}
pub(crate) struct OwnedDirectory(pub PathBuf);
impl Drop for OwnedDirectory { fn drop(&mut self) { if !self.0.as_os_str().is_empty() { let _ = std::fs::remove_dir_all(&self.0); } } }
pub(crate) fn job_directory(root: &Path) -> anyhow::Result<OwnedDirectory> {
    let jobs = root.canonicalize()?.join("updates/jobs"); private_dir(&jobs)?;
    let mut random = [0; 16]; ring::rand::SecureRandom::fill(&ring::rand::SystemRandom::new(), &mut random).map_err(|_| anyhow::anyhow!("OS randomness unavailable"))?;
    let path = jobs.join(hex(&random)); std::fs::create_dir(&path)?; private_dir(&path)?; Ok(OwnedDirectory(path))
}
fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }
fn hash_file(path: &Path, limit: u64) -> anyhow::Result<String> {
    let meta = std::fs::symlink_metadata(path)?;
    ensure!(meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= limit, "Unexpected update file: {}", path.display());
    let mut reader = std::fs::File::open(path)?; let mut buffer = [0; 65536]; let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    loop { let n = reader.read(&mut buffer)?; if n == 0 { break } hash.update(&buffer[..n]); }
    Ok(hex(hash.finish().as_ref()))
}
fn tag_valid(tag: &str) -> anyhow::Result<()> {
    ensure!(tag.len() <= 64, "Unsupported update tag"); let version = super::version(tag)?;
    ensure!(version.pre.is_empty(), "Only stable releases can be installed"); Ok(())
}
pub(super) fn platform(release: &Release) -> anyhow::Result<&'static str> {
    tag_valid(&release.tag)?;
    for (platform, ext) in [("windows-x86_64", "zip"), ("linux-x86_64", "tar.gz")] {
        let name = format!("yo3-revival-{}-{platform}.{ext}", release.tag);
        if release.asset.name == name {
            ensure!(release.asset.url == format!("{}/releases/download/{}/{}", super::REPOSITORY, release.tag, name), "Unexpected update download source");
            return Ok(platform)
        }
    }
    anyhow::bail!("Unsupported update bundle")
}
fn relative(name: &Path, prefix: &str) -> anyhow::Result<PathBuf> {
    let text = name.to_str().context("Unsupported archive path")?;
    ensure!(!text.contains(['\\', ':', '\0', '\r', '\n']) && text.len() < 4096, "Unsafe archive path");
    for part in text.trim_end_matches('/').split('/') {
        let base = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        ensure!(!part.is_empty() && part != "." && part != ".." && !part.ends_with([' ', '.']), "Unsafe archive path");
        ensure!(!["CON", "PRN", "AUX", "NUL"].contains(&base.as_str()) && !(base.len() == 4 && (base.starts_with("COM") || base.starts_with("LPT")) && matches!(base.as_bytes()[3], b'1'..=b'9')), "Reserved archive path");
    }
    let mut parts = name.components(); ensure!(parts.next() == Some(Component::Normal(prefix.as_ref())), "Update has an unexpected root folder");
    let mut result = PathBuf::new();
    for part in parts { match part { Component::Normal(part) => result.push(part), _ => anyhow::bail!("Unsafe archive path") } }
    if let Some(first) = result.components().next() {
        let first = first.as_os_str().to_string_lossy().to_ascii_lowercase();
        ensure!(!["data", "logs", "profiles", "launcher.db", "server.db", "server.key", "client.json", "yo3.exe", "yo3.ini"].contains(&first.as_str()), "Update contains user data or original game files");
    }
    ensure!(result.extension().is_none_or(|s| !s.eq_ignore_ascii_case("ypk")), "Update contains original game archives");
    Ok(result)
}
struct Budget<'a> { bytes: u64, files: usize, cancelled: &'a dyn Fn() -> bool }
impl Budget<'_> {
    fn add(&mut self, size: u64) -> anyhow::Result<()> {
        ensure!(!(self.cancelled)(), "Update cancelled"); self.files += 1;
        ensure!(self.files <= MAX_FILES && size <= MAX_UNPACK.saturating_sub(self.bytes), "Update expands beyond the supported size"); self.bytes += size; Ok(())
    }
}
fn unpack_file(mut source: impl Read, target: &Path, length: u64, executable: bool) -> anyhow::Result<()> {
    if let Some(parent) = target.parent() { private_dir(parent)?; }
    let mut file = private_create(target)?;
    ensure!(std::io::copy(&mut source.by_ref().take(length+1), &mut file)? == length, "Archive file length changed while extracting");
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; file.set_permissions(std::fs::Permissions::from_mode(if executable { 0o700 } else { 0o600 }))?; }
    #[cfg(not(unix))] let _ = executable;
    Ok(())
}
fn unpack(archive: &Path, destination: &Path, prefix: &str, windows: bool, cancelled: &dyn Fn() -> bool) -> anyhow::Result<()> {
    let mut budget = Budget { bytes: 0, files: 0, cancelled };
    if windows {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?; let raw_name = entry.name()?; let path = relative(Path::new(raw_name.as_ref()), prefix)?;
            let mode = entry.unix_mode().unwrap_or(0); let kind = mode & 0o170000;
            ensure!(kind == 0 || kind == 0o100000 || kind == 0o040000, "Update contains a link or special file");
            let length = entry.size(); budget.add(length)?;
            if path.as_os_str().is_empty() { ensure!(entry.is_dir(), "Invalid archive root"); continue }
            let target = destination.join(path);
            if entry.is_dir() { private_dir(&target)?; } else { unpack_file(&mut entry, &target, length, mode & 0o111 != 0)?; }
        }
    } else {
        let gzip = flate2::read::GzDecoder::new(std::fs::File::open(archive)?); let mut tar = tar::Archive::new(gzip);
        for entry in tar.entries()? {
            let mut entry = entry?; let path = relative(&entry.path()?, prefix)?; let kind = entry.header().entry_type();
            ensure!(kind.is_file() || kind.is_dir(), "Update contains a link or special file");
            let length = entry.size(); budget.add(length)?;
            if path.as_os_str().is_empty() { ensure!(kind.is_dir(), "Invalid archive root"); continue }
            let target = destination.join(path);
            if kind.is_dir() { private_dir(&target)?; } else { let executable = entry.header().mode()? & 0o111 != 0; unpack_file(&mut entry, &target, length, executable)?; }
        }
    }
    Ok(())
}
fn bundle(directory: &Path, tag: &str, platform: &str) -> anyhow::Result<(String, String)> {
    let manifest = directory.join("bundle.json"); let manifest_sha = hash_file(&manifest, 65536)?;
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest)?)?;
    ensure!(value["schema"] == 1 && value["release"] == tag, "Update manifest has the wrong format or version");
    for key in ["protocol", "mod_abi"] { ensure!(value[key].as_u64().is_some_and(|n| n > 0 && n <= 65535), "Update manifest lacks compatibility information"); }
    let components = value["components"].as_object().context("Update manifest has no components")?;
    ensure!(components.len() <= 32, "Update has too many components");
    let windows = platform == "windows-x86_64"; let ext = if windows { ".exe" } else { "" };
    for (role, name) in [("desktop", format!("yo3-revival{ext}")), ("server", format!("yo3-server{ext}")), ("launcher", format!("yo3-launcher{ext}")),
        ("injector", "yo3-auth-launch.exe".into()), ("connection_mod", "yo3-auth.dll".into()), ("loader", "yo3-bootstrap.dll".into())] {
        ensure!(components.get(role).is_some_and(|c| c["file"] == name), "Update is missing its {role} component");
    }
    if !windows { ensure!(components.get("direct_launcher").is_some_and(|c| c["file"] == "yo3-launcher-windows.exe"), "Linux update is missing its Windows connection helper"); }
    for component in components.values() {
        let name = component["file"].as_str().context("Missing component file name")?;
        ensure!(!name.contains(['/', '\\', ':']) && name != "." && name != "..", "Unsafe component path");
        let expected = component["sha256"].as_str().context("Missing component checksum")?;
        ensure!(hash_file(&directory.join(name), MAX_DOWNLOAD)? == expected, "Update component checksum failed: {name}");
    }
    for name in ["engine/duelcore.ts", "engine/package.json", "engine/vendor/cardinfo.json", "engine/node_modules/ocgcore-wasm/package.json", "engine/vendor/BabelCDB/cards.cdb", "engine/vendor/CardScripts/utility.lua", if windows { "engine/node/node.exe" } else { "engine/node/node" }] {
        let meta = std::fs::symlink_metadata(directory.join(name)).with_context(|| format!("Update is incomplete: {name}"))?;
        ensure!(meta.is_file() && !meta.file_type().is_symlink(), "Unexpected update engine component");
    }
    Ok((manifest_sha, components["desktop"]["sha256"].as_str().unwrap().into()))
}
pub fn stage_archive(root: &Path, release: &Release, archive: &Path) -> anyhow::Result<Installed> { stage_with(root, release, archive, &|| false) }
pub(crate) fn stage_with(root: &Path, release: &Release, archive: &Path, cancelled: &dyn Fn() -> bool) -> anyhow::Result<Installed> {
    let platform = platform(release)?;
    ensure!(std::fs::metadata(archive)?.len() == release.asset.bytes && release.asset.bytes <= MAX_DOWNLOAD, "Update download length does not match GitHub");
    ensure!(hash_file(archive, MAX_DOWNLOAD)? == release.asset.sha256, "Update download checksum does not match GitHub");
    private_dir(root)?; let root = root.canonicalize()?; let installs = root.join("updates/installs"); private_dir(&installs)?;
    let directory = installs.join(format!("{}-{platform}", release.tag));
    if std::fs::symlink_metadata(&directory).is_ok() {
        let receipt = directory.join(RECEIPT); hash_file(&receipt, 65536).context("Unrecognized existing update folder")?;
        let previous: Installed = serde_json::from_slice(&std::fs::read(receipt)?)?;
        ensure!(previous.tag == release.tag && previous.platform == platform && previous.directory == directory, "Existing update folder has an invalid receipt");
        ensure!(previous.archive_sha256 == release.asset.sha256, "A different update already occupies this version folder"); validate(&root, &previous)?; return Ok(previous)
    }
    let job = job_directory(&root)?; let unpacked = job.0.join("bundle"); private_dir(&unpacked)?;
    unpack(archive, &unpacked, &format!("yo3-revival-{}-{platform}", release.tag), platform == "windows-x86_64", cancelled)?;
    let (manifest_sha256, desktop_sha256) = bundle(&unpacked, &release.tag, platform)?;
    let installed = Installed { tag: release.tag.clone(), directory: directory.clone(), platform: platform.into(), manifest_sha256, desktop_sha256, archive_sha256: release.asset.sha256.clone() };
    private_create(&unpacked.join(RECEIPT))?.write_all(&serde_json::to_vec_pretty(&installed)?)?;
    ensure!(!cancelled(), "Update cancelled"); std::fs::rename(unpacked, &directory)?; validate(&root, &installed)?; Ok(installed)
}
pub fn validate(root: &Path, installed: &Installed) -> anyhow::Result<()> {
    tag_valid(&installed.tag)?; ensure!(["linux-x86_64", "windows-x86_64"].contains(&installed.platform.as_str()), "Unsupported installed platform");
    let expected = root.canonicalize()?.join("updates/installs").join(format!("{}-{}", installed.tag, installed.platform));
    ensure!(installed.directory == expected && !std::fs::symlink_metadata(&expected)?.file_type().is_symlink(), "Update is outside managed application storage");
    ensure!(expected.canonicalize()? == expected, "Update storage contains a symbolic link");
    let receipt = expected.join(RECEIPT); hash_file(&receipt, 65536)?;
    let saved: Installed = serde_json::from_slice(&std::fs::read(receipt)?)?; ensure!(saved == *installed, "Installed update receipt changed");
    let (manifest, desktop) = bundle(&expected, &installed.tag, &installed.platform)?;
    ensure!(manifest == installed.manifest_sha256 && desktop == installed.desktop_sha256, "Installed update no longer matches its verified release"); Ok(())
}
pub fn active(root: &Path) -> anyhow::Result<Option<Installed>> {
    let db = crate::LauncherDb::open(root)?; db.conn.execute_batch("CREATE TABLE IF NOT EXISTS update_state (id INTEGER PRIMARY KEY CHECK(id IN (1,2)), body TEXT NOT NULL)")?;
    use rusqlite::OptionalExtension;
    let body: Option<String> = db.conn.query_row("SELECT body FROM update_state WHERE id=1", [], |r| r.get(0)).optional()?;
    body.map(|body| serde_json::from_str(&body).map_err(Into::into)).transpose()
}
pub(crate) fn promote(root: &Path, installed: &Installed) -> anyhow::Result<()> {
    validate(root, installed)?;
    if active(root)?.as_ref() == Some(installed) { return Ok(()) }
    if let Some(previous) = active(root)? { ensure!(super::version(&installed.tag)?.cmp_precedence(&super::version(&previous.tag)?).is_ge(), "Refusing an update downgrade"); }
    let mut db = crate::LauncherDb::open(root)?; let transaction = db.conn.transaction()?;
    transaction.execute("INSERT INTO update_state(id,body) SELECT 2,body FROM update_state WHERE id=1 ON CONFLICT(id) DO UPDATE SET body=excluded.body", [])?;
    transaction.execute("INSERT INTO update_state VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET body=excluded.body", [serde_json::to_string(installed)?])?;
    transaction.commit()?; Ok(())
}
