//! Shared static preflight for GUI, CLI and direct-start repair. No credentials or network operations here.
use std::path::Path;
use anyhow::{Context, ensure};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct ModInfo { pub abi: u16, pub release: String, pub commit: String }

fn u16_at(bytes: &[u8], at: usize) -> anyhow::Result<u16> {
    Ok(u16::from_le_bytes(bytes.get(at..at+2).context("truncated PE header")?.try_into()?))
}
fn u32_at(bytes: &[u8], at: usize) -> anyhow::Result<u32> {
    Ok(u32::from_le_bytes(bytes.get(at..at+4).context("truncated PE header")?.try_into()?))
}
fn pe32(path: &Path) -> anyhow::Result<(Vec<u8>, usize, usize)> {
    ensure!(std::fs::metadata(path).with_context(|| format!("Required component not found: {}. Repair the installation or select the matching file.", path.display()))?.len() <= 32 * 1024 * 1024, "component is unexpectedly large");
    let bytes = std::fs::read(path)?;
    ensure!(bytes.get(..2) == Some(b"MZ"), "{} is not a Windows executable", path.display());
    let pe = u32_at(&bytes, 0x3c)? as usize;
    ensure!(bytes.get(pe..pe+4) == Some(b"PE\0\0"), "{} has an invalid PE signature", path.display());
    ensure!(u16_at(&bytes, pe+4)? == 0x14c, "{} must be a 32-bit x86 component", path.display());
    let count = u16_at(&bytes, pe+6)? as usize;
    let optional = u16_at(&bytes, pe+20)? as usize;
    ensure!((1..=96).contains(&count) && optional >= 96 && u16_at(&bytes, pe+24)? == 0x10b, "unsupported PE layout");
    let sections = pe.checked_add(24 + optional).context("invalid PE offsets")?;
    ensure!(bytes.get(sections..sections+40*count).is_some(), "truncated PE sections");
    Ok((bytes, sections, count))
}
fn text(bytes: &[u8]) -> anyhow::Result<String> {
    let end = bytes.iter().position(|b| *b == 0).context("unterminated component metadata")?;
    let s = std::str::from_utf8(&bytes[..end])?;
    ensure!(!s.is_empty() && !s.chars().any(char::is_control), "invalid component metadata"); Ok(s.into())
}
pub fn inspect_mod(path: &Path) -> anyhow::Result<ModInfo> {
    let result = || -> anyhow::Result<ModInfo> {
        let (bytes, sections, count) = pe32(path)?;
        let found: Vec<_> = (0..count).map(|i| sections + 40*i).filter(|at| &bytes[*at..*at+8] == b".yo3\0\0\0\0").collect();
        ensure!(found.len() == 1, "missing/ambiguous revival DLL metadata");
        let section = found[0]; let length = u32_at(&bytes, section+16)? as usize; let start = u32_at(&bytes, section+20)? as usize;
        ensure!(length >= 118, "truncated revival DLL metadata");
        let data = bytes.get(start..start.checked_add(length).context("invalid metadata offsets")?).context("truncated DLL section")?;
        ensure!(&data[..8] == b"YO3MOD02", "unsupported connection DLL metadata");
        let abi = u32_at(data, 8)?; ensure!(abi == yo3_tunnel::MOD_ABI as u32, "Connection DLL interface {abi} does not match required interface {}", yo3_tunnel::MOD_ABI);
        let release = text(&data[12..77])?; ensure!(release == yo3_tunnel::BUILD_VERSION, "Connection DLL release {release} does not match application {}. Repair with files from the same bundle.", yo3_tunnel::BUILD_VERSION);
        Ok(ModInfo { abi: abi as u16, release, commit: text(&data[77..118])? })
    };
    result().with_context(|| format!("Connection DLL validation failed: {}", path.display()))
}
pub fn hash(bytes: &[u8]) -> String { ring::digest::digest(&ring::digest::SHA256, bytes).as_ref().iter().map(|b| format!("{b:02x}")).collect() }
pub fn preflight(game: &Path, injector: &Path, dll: &Path) -> anyhow::Result<ModInfo> {
    pe32(game).context("Unsupported or incomplete game executable")?;
    pe32(injector).context("Missing or invalid 32-bit injector")?;
    let info = inspect_mod(dll)?;
    let manifest = injector.parent().unwrap_or(Path::new(".")).join("bundle.json");
    if manifest.exists() {
        let bytes = std::fs::read(&manifest)?; ensure!(bytes.len() <= 65536, "invalid bundle manifest size");
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        ensure!(value["release"] == yo3_tunnel::BUILD_VERSION && value["mod_abi"] == yo3_tunnel::MOD_ABI, "bundle manifest is incompatible; extract the complete matching release");
        for (role, file) in [("injector", injector), ("connection_mod", dll)] {
            let expected = value["components"][role]["sha256"].as_str().context("incomplete bundle manifest")?;
            ensure!(hash(&std::fs::read(file)?) == expected, "{} differs from the release manifest. Repair or select its original file.", file.display());
        }
    } else { ensure!(yo3_tunnel::BUILD_VERSION.ends_with("-dev"), "Bundle manifest is missing. Extract the complete release again."); }
    Ok(info)
}
