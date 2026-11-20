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
    let (sections, count) = pe32_layout(&bytes, &path.display().to_string())?;
    Ok((bytes, sections, count))
}
/// A 32-bit PE's section table: where it starts and how many sections there are.
fn pe32_layout(bytes: &[u8], name: &str) -> anyhow::Result<(usize, usize)> {
    ensure!(bytes.get(..2) == Some(b"MZ"), "{name} is not a Windows executable");
    let pe = u32_at(bytes, 0x3c)? as usize;
    ensure!(bytes.get(pe..pe+4) == Some(b"PE\0\0"), "{name} has an invalid PE signature");
    ensure!(u16_at(bytes, pe+4)? == 0x14c, "{name} must be a 32-bit x86 component");
    let count = u16_at(bytes, pe+6)? as usize;
    let optional = u16_at(bytes, pe+20)? as usize;
    ensure!((1..=96).contains(&count) && optional >= 96 && u16_at(bytes, pe+24)? == 0x10b, "unsupported PE layout");
    let sections = pe.checked_add(24 + optional).context("invalid PE offsets")?;
    ensure!(bytes.get(sections..sections+40*count).is_some(), "truncated PE sections");
    Ok((sections, count))
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
/// What a file says about itself when it is one of YO3 Revival's own game-folder loaders (`winmm.dll`, built from
/// `tools/mod/loader.c`): any release's, not only this application's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loader { pub abi: u32, pub release: String, pub commit: String }
/// Whether `bytes` is a YO3 Revival loader, of any release (#65: a player's `winmm.dll` was ours, from the v0.1.3 bundle,
/// and was called foreign for lack of its receipt). The proof is the `.yo3` section every loader and connection DLL
/// carries (`YO3MOD02`, the interface, release and commit) together with the loader's own export `Yo3LoaderInfo`,
/// which the connection DLL (`Yo3ModInfo`) doesn't have. Another mod can copy both; this recognises our file for the
/// player's messages and repairs, never for trust: a loader still has to match the bundle's hash to be installed.
pub fn recognize_loader(bytes: &[u8]) -> Option<Loader> {
    if bytes.len() > 32 * 1024 * 1024 { return None }
    let (sections, count) = pe32_layout(bytes, "file").ok()?;
    let mut found = (0..count).map(|i| sections + 40*i).filter(|at| bytes.get(*at..*at+8) == Some(b".yo3\0\0\0\0".as_slice()));
    let (section, None) = (found.next()?, found.next()) else { return None };
    let length = u32_at(bytes, section+16).ok()? as usize; let start = u32_at(bytes, section+20).ok()? as usize;
    if length < 118 { return None }
    let data = bytes.get(start..start.checked_add(length)?)?;
    if &data[..8] != b"YO3MOD02" { return None }
    if !bytes.windows(13).any(|w| w == b"Yo3LoaderInfo") { return None }
    Some(Loader { abi: u32_at(data, 8).ok()?, release: text(&data[12..77]).ok()?, commit: text(&data[77..118]).ok()? })
}
/// A file that passes [`recognize_loader`] as `release`'s loader, for other crates' tests.
#[doc(hidden)]
pub fn crafted_loader_for_tests(release: &str) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x400];
    bytes[..2].copy_from_slice(b"MZ"); bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    bytes[0x80..0x84].copy_from_slice(b"PE\0\0"); bytes[0x84..0x86].copy_from_slice(&0x14cu16.to_le_bytes());
    bytes[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
    bytes[0x94..0x96].copy_from_slice(&224u16.to_le_bytes()); bytes[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes());
    let at = 0x80 + 24 + 224;
    bytes[at..at+8].copy_from_slice(b".yo3\0\0\0\0");
    bytes[at+16..at+20].copy_from_slice(&200u32.to_le_bytes()); bytes[at+20..at+24].copy_from_slice(&0x300u32.to_le_bytes());
    bytes.resize(0x300 + 200, 0);
    bytes[0x300..0x308].copy_from_slice(b"YO3MOD02"); bytes[0x308..0x30c].copy_from_slice(&3u32.to_le_bytes());
    bytes[0x30c..0x30c + release.len()].copy_from_slice(release.as_bytes());
    bytes[0x300 + 77..0x300 + 77 + 4].copy_from_slice(b"test");
    bytes.extend_from_slice(b"Yo3LoaderInfo");
    bytes
}
pub fn hash(bytes: &[u8]) -> String { ring::digest::digest(&ring::digest::SHA256, bytes).as_ref().iter().map(|b| format!("{b:02x}")).collect() }
/// Check the same four executable sites guarded by the required DLL, using PE RVAs rather than file offsets.
/// A version label or a valid PE header alone does not establish that this game's hooks are supported.
pub fn inspect_game(path: &Path) -> anyhow::Result<String> {
    let check = || -> anyhow::Result<String> {
        let (bytes, sections, count) = pe32(path)?;
        let pe = u32_at(&bytes, 0x3c)? as usize;
        ensure!(u32_at(&bytes, pe+24+28)? == 0x400000, "Unexpected game image base");
        ensure!(u32_at(&bytes, pe+24+56)? >= 0x1159ef, "Game image is incomplete");
        for (rva, expected, name) in [(0x6964bu32, [0xe8,0x50,0x6c,0x08,0], "login"),
            (0xf44bd, [0xe8,0x3e,0x05,0,0], "starter dispatcher"),
            (0x114212, [0xe8,0x79,0,0,0], "duel dispatcher"),
            (0x1159ea, [0xb8,0x30,0x5a,0x51,0], "duel update")] {
            let found = (0..count).find_map(|i| {
                let section = sections+40*i;
                let offset = rva.checked_sub(u32_at(&bytes, section+12).ok()?)?;
                let raw = u32_at(&bytes, section+16).ok()?;
                if offset.checked_add(5)? > raw || u32_at(&bytes, section+36).ok()? & 0x20000000 == 0 { return None }
                let at = (u32_at(&bytes, section+20).ok()? as usize).checked_add(offset as usize)?;
                bytes.get(at..at.checked_add(5)?)
            });
            ensure!(found == Some(expected.as_slice()), "Unsupported {name} hook layout at 0x{:X}", 0x400000+rva);
        }
        Ok(hash(&bytes))
    };
    check().with_context(|| format!("Unsupported or incomplete game executable: {}. Use the supported original Yu-Gi-Oh! ONLINE 3 v1.153 installation; changing Revival helper files will not repair this game build.", path.display()))
}
pub fn preflight(game: &Path, injector: &Path, dll: &Path) -> anyhow::Result<ModInfo> {
    inspect_game(game)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal 32-bit PE with a `.yo3` section holding `YO3MOD02`, `abi`, `release` and `commit`, and `export` in its body.
    fn crafted(magic: &[u8; 8], abi: u32, release: &str, commit: &str, export: &str, sections: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x400];
        bytes[..2].copy_from_slice(b"MZ"); bytes[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0"); bytes[0x84..0x86].copy_from_slice(&0x14cu16.to_le_bytes());
        bytes[0x86..0x88].copy_from_slice(&(sections as u16).to_le_bytes());
        bytes[0x94..0x96].copy_from_slice(&224u16.to_le_bytes()); bytes[0x98..0x9a].copy_from_slice(&0x10bu16.to_le_bytes());
        let table = 0x80 + 24 + 224;
        for i in 0..sections {
            let at = table + 40 * i;
            bytes[at..at+8].copy_from_slice(b".yo3\0\0\0\0");
            bytes[at+16..at+20].copy_from_slice(&200u32.to_le_bytes()); bytes[at+20..at+24].copy_from_slice(&0x300u32.to_le_bytes());
        }
        bytes.resize(0x300 + 200, 0);
        bytes[0x300..0x308].copy_from_slice(magic); bytes[0x308..0x30c].copy_from_slice(&abi.to_le_bytes());
        bytes[0x30c..0x30c + release.len()].copy_from_slice(release.as_bytes());
        bytes[0x300 + 77..0x300 + 77 + commit.len()].copy_from_slice(commit.as_bytes());
        bytes.extend_from_slice(export.as_bytes());
        bytes
    }

    #[test]
    fn a_loader_of_any_release_is_recognised_without_its_receipt() {
        let old = crafted(b"YO3MOD02", 3, "v0.1.3", "56ba97561f92", "Yo3LoaderInfo", 1);
        assert_eq!(recognize_loader(&old), Some(Loader { abi: 3, release: "v0.1.3".into(), commit: "56ba97561f92".into() }));
        assert_eq!(recognize_loader(&crafted_loader_for_tests("v0.1.9")).map(|l| l.release), Some("v0.1.9".into()));
        let other = crafted(b"YO3MOD02", 9, "v9.9.9", "abc", "Yo3LoaderInfo", 1);
        assert_eq!(recognize_loader(&other).map(|l| l.release), Some("v9.9.9".into()), "not only this application's release");
    }

    #[test]
    fn other_files_are_not_taken_for_a_loader() {
        assert_eq!(recognize_loader(b"someone else's"), None);
        assert_eq!(recognize_loader(&crafted(b"YO3MOD01", 3, "v0.1.3", "c", "Yo3LoaderInfo", 1)), None, "another metadata version");
        assert_eq!(recognize_loader(&crafted(b"YO3MOD02", 3, "v0.1.3", "c", "Yo3ModInfo", 1)), None, "the connection DLL isn't the loader");
        assert_eq!(recognize_loader(&crafted(b"YO3MOD02", 3, "v0.1.3", "c", "Yo3LoaderInfo", 2)), None, "an ambiguous section table");
        let mut truncated = crafted(b"YO3MOD02", 3, "v0.1.3", "c", "Yo3LoaderInfo", 1); truncated.truncate(0x340);
        assert_eq!(recognize_loader(&truncated), None);
    }

    /// The file a player attached to #65, read only (never run, never copied into the repository): YO3_ISSUE65_WINMM names
    /// it. It was our own v0.1.3 loader. Skipped when the variable is not set.
    #[test]
    fn the_winmm_dll_from_issue_65_is_our_v0_1_3_loader() {
        let Some(path) = std::env::var_os("YO3_ISSUE65_WINMM") else { return };
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(hash(&bytes), "1fa64875826f1fbfdf62f42fa1e9bc13c49bc993811fb6dd17cb1d4b19ff66d4");
        let loader = recognize_loader(&bytes).expect("our loader");
        assert_eq!((loader.release.as_str(), loader.commit.as_str(), loader.abi), ("v0.1.3", "56ba97561f92", 3));
    }
}
