//! Signed updates (#32): the release manifest's Ed25519 signature, checked before an update is downloaded.
//!
//! The Release workflow writes `SHA256SUMS` (`sha256sum` lines: `<hex>  <file name>`) over the release's files, signs
//! it with the project's private key (the repository secret `UPDATE_SIGNING_KEY`;
//! `openssl pkeyutl -sign -rawin -inkey KEY -in SHA256SUMS -out SHA256SUMS.sig`: 64 raw bytes), checks the signature
//! with the public key below and attaches both files to the release. `download::prepare` fetches both before the
//! archive and refuses the update unless the signature is the project's and the manifest lists the same SHA-256 that
//! GitHub reports for the download (which the download is then checked against).
//!
//! **A release without `SHA256SUMS.sig` (or without `SHA256SUMS`) is refused.** The launcher is only ever offered a
//! release newer than itself, and every release since this key was built in is signed by the workflow that made it, so
//! an unsigned newer release is a broken or substituted one, never an old one. Releases before signing began (v0.0.21
//! and older) are never offered to a launcher that checks signatures; they themselves don't check, so they still
//! update to a signed release.
//!
//! `tools/release/test-packaged-update.sh` checks a signed manifest end to end with a key it makes for the test, never this one.
use anyhow::{Context, ensure};
use super::Release;

/// The manifest listing each release file's SHA-256, attached to the release.
pub const MANIFEST: &str = "SHA256SUMS";
/// Its Ed25519 signature (64 raw bytes), attached beside it.
pub const SIGNATURE: &str = "SHA256SUMS.sig";
/// The largest manifest read.
pub const MAX_MANIFEST: usize = 64 * 1024;

/// The project's release-signing public key (raw Ed25519, 32 bytes; `openssl pkey -pubout -outform DER | tail -c 32`).
/// The private key is only the repository secret `UPDATE_SIGNING_KEY`. `None` would turn the check off.
pub const PUBLIC_KEY: Option<[u8; 32]> = Some([
    0xdb, 0xb2, 0x5e, 0xca, 0x8c, 0x2c, 0xef, 0xb9, 0x41, 0xad, 0xdd, 0x99, 0xe3, 0xa8, 0x51, 0xf7,
    0x7f, 0xcc, 0x78, 0x0c, 0xa5, 0x06, 0x02, 0x60, 0xf3, 0x26, 0x40, 0xd1, 0x9e, 0x09, 0x98, 0x7c,
]);

/// Whether updates must carry a valid signature (once `PUBLIC_KEY` is set).
pub const fn required() -> bool { PUBLIC_KEY.is_some() }

/// A raw Ed25519 public key from its 64 hex digits.
pub fn parse_key(hex: &str) -> Option<[u8; 32]> {
    let hex = hex.trim();
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) { return None }
    let mut key = [0; 32];
    for (i, byte) in key.iter_mut().enumerate() { *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()?; }
    Some(key)
}

/// The manifest's signature is `public_key`'s.
pub fn verify(manifest: &[u8], signature: &[u8], public_key: &[u8; 32]) -> anyhow::Result<()> {
    ensure!(manifest.len() <= MAX_MANIFEST, "The update's manifest is too large");
    ensure!(signature.len() == 64, "The update's signature is not a signature");
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public_key).verify(manifest, signature)
        .map_err(|_| anyhow::anyhow!("The update's signature does not match; it was not installed"))
}

/// The SHA-256 the manifest lists for `name` (exactly one line).
pub fn listed(manifest: &[u8], name: &str) -> anyhow::Result<String> {
    let text = std::str::from_utf8(manifest).context("The update's manifest is not text")?;
    let found: Vec<&str> = text.lines().filter_map(|line| {
        let (hash, file) = line.split_once(char::is_whitespace)?;
        (file.trim_start().trim_start_matches('*') == name).then_some(hash)
    }).collect();
    ensure!(found.len() == 1, "The update's manifest does not list {name} once");
    let hash = found[0];
    ensure!(hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()), "The update's manifest has an invalid checksum");
    Ok(hash.to_ascii_lowercase())
}

/// The signed manifest vouches for this release's download: the signature is ours and it lists the same SHA-256 that
/// GitHub reported (which the download is then checked against).
pub fn check(release: &Release, manifest: &[u8], signature: &[u8], public_key: &[u8; 32]) -> anyhow::Result<()> {
    verify(manifest, signature, public_key)?;
    ensure!(listed(manifest, &release.asset.name)? == release.asset.sha256, "The update's signed checksum differs from GitHub's; it was not installed");
    Ok(())
}

/// The release's signature files as fetched: `None` when the release doesn't have that file (HTTP 404). A release
/// without them is refused (see the module's notes).
pub fn vouch(release: &Release, manifest: Option<&[u8]>, signature: Option<&[u8]>, public_key: &[u8; 32]) -> anyhow::Result<()> {
    let (Some(manifest), Some(signature)) = (manifest, signature) else {
        anyhow::bail!("Update {} isn't signed (its {} is missing), so it wasn't installed. You can keep playing; if you trust \
            it, download it yourself from the releases page.", release.tag.trim_start_matches('v'), if manifest.is_none() { MANIFEST } else { SIGNATURE })
    };
    check(release, manifest, signature, public_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Asset;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    /// A throwaway key made for this test only; the project's key is never in the repository.
    fn test_key() -> Ed25519KeyPair {
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
        Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap()
    }
    fn public(key: &Ed25519KeyPair) -> [u8; 32] { key.public_key().as_ref().try_into().unwrap() }
    fn release(sha256: &str) -> Release {
        let name = "yo3-revival-desktop-v0.2.1-linux-x86_64.tar.gz";
        Release { tag: "v0.2.1".into(), url: String::new(), notes: String::new(),
            asset: Asset { name: name.into(), url: String::new(), bytes: 1, sha256: sha256.into() } }
    }

    #[test]
    fn the_project_key_is_built_in_and_updates_must_be_signed() {
        assert!(required());
        assert_eq!(PUBLIC_KEY, parse_key("dbb25eca8c2cefb941addd99e3a851f77fcc780ca5060260f32640d19e09987c"));
        assert!(ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, PUBLIC_KEY.unwrap()).verify(b"x", &[0; 64]).is_err());
        assert!(parse_key("dbb2").is_none() && parse_key(&"zz".repeat(32)).is_none());
    }

    #[test]
    fn a_release_without_its_signature_or_manifest_is_refused_with_a_plain_reason() {
        let key = test_key();
        let hash = "ab".repeat(32);
        let manifest = format!("{hash}  yo3-revival-desktop-v0.2.1-linux-x86_64.tar.gz\n");
        let signature = key.sign(manifest.as_bytes());
        vouch(&release(&hash), Some(manifest.as_bytes()), Some(signature.as_ref()), &public(&key)).unwrap();
        let error = vouch(&release(&hash), Some(manifest.as_bytes()), None, &public(&key)).unwrap_err().to_string();
        assert!(error.starts_with("Update 0.2.1 isn't signed (its SHA256SUMS.sig is missing)"), "{error}");
        let error = vouch(&release(&hash), None, None, &public(&key)).unwrap_err().to_string();
        assert!(error.contains("its SHA256SUMS is missing"), "{error}");
    }

    #[test]
    fn a_signed_manifest_vouches_for_the_download_and_a_bad_signature_is_refused() {
        let key = test_key();
        let hash = "ab".repeat(32);
        let manifest = format!("{}  yo3-revival-desktop-v0.2.1-windows-x86_64.zip\n{hash}  yo3-revival-desktop-v0.2.1-linux-x86_64.tar.gz\n", "cd".repeat(32));
        let signature = key.sign(manifest.as_bytes());
        check(&release(&hash), manifest.as_bytes(), signature.as_ref(), &public(&key)).unwrap();

        let error = check(&release(&hash), manifest.as_bytes(), signature.as_ref(), &public(&test_key())).unwrap_err();
        assert!(error.to_string().contains("signature does not match"), "another key's signature: {error}");
        let mut flipped = signature.as_ref().to_vec();
        flipped[0] ^= 1;
        assert!(check(&release(&hash), manifest.as_bytes(), &flipped, &public(&key)).is_err(), "a changed signature");
        let changed = manifest.replace(&hash, &"ef".repeat(32));
        assert!(check(&release(&"ef".repeat(32)), changed.as_bytes(), signature.as_ref(), &public(&key)).is_err(), "a changed manifest");
        assert!(check(&release(&hash), manifest.as_bytes(), &signature.as_ref()[..63], &public(&key)).is_err(), "a truncated signature");
        let error = check(&release(&"00".repeat(32)), manifest.as_bytes(), signature.as_ref(), &public(&key)).unwrap_err();
        assert!(error.to_string().contains("differs from GitHub"), "GitHub reports another checksum: {error}");
    }

    #[test]
    fn the_manifest_must_list_the_download_exactly_once() {
        let hash = "ab".repeat(32);
        assert_eq!(listed(format!("{hash} *file.zip\n").as_bytes(), "file.zip").unwrap(), hash, "sha256sum's binary marker");
        assert!(listed(b"", "file.zip").is_err());
        assert!(listed(format!("{hash}  file.zip\n{hash}  file.zip\n").as_bytes(), "file.zip").is_err());
        assert!(listed(b"nothex  file.zip\n", "file.zip").is_err());
        assert!(listed(format!("{hash}  other-file.zip\n").as_bytes(), "file.zip").is_err());
    }
}
