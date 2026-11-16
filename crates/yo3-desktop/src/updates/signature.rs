//! SIGNING SEAM (#32): the release manifest's signature, checked before an update is downloaded.
//!
//! **Off until the project's signing key exists.** `PUBLIC_KEY` is `None`, so `download::prepare` skips this check and
//! an update is verified only against the SHA-256 GitHub reports (as before #32). To turn it on:
//!
//! 1. The owner makes an Ed25519 key pair (`openssl genpkey -algorithm ed25519 -out release.key`), keeps the private
//!    key as a CI secret, and puts the 32-byte public key here
//!    (`openssl pkey -in release.key -pubout -outform DER | tail -c 32 | xxd -i`).
//! 2. The Release workflow writes `SHA256SUMS` (`sha256sum` lines: `<hex>  <file name>`) for the release's files and
//!    signs it (`openssl pkeyutl -sign -rawin -inkey release.key -in SHA256SUMS -out SHA256SUMS.sig`: 64 raw bytes),
//!    and attaches both to the release.
//! 3. `tools/test-packaged-update.sh` gets a signed manifest to check (the test key made in the test, never this one).
//!
//! The format is plain Ed25519 over the manifest's bytes, so `ring` (already a dependency) checks it.
use anyhow::{Context, ensure};
use super::Release;

/// The manifest listing each release file's SHA-256, attached to the release.
pub const MANIFEST: &str = "SHA256SUMS";
/// Its Ed25519 signature (64 raw bytes), attached beside it.
pub const SIGNATURE: &str = "SHA256SUMS.sig";
/// The largest manifest read.
pub const MAX_MANIFEST: usize = 64 * 1024;

/// The project's release-signing public key (raw Ed25519, 32 bytes). `None`: no key exists yet, nothing is checked.
pub const PUBLIC_KEY: Option<[u8; 32]> = None;

/// Whether updates must carry a valid signature (only once `PUBLIC_KEY` is set).
pub const fn required() -> bool { PUBLIC_KEY.is_some() }

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
    fn the_seam_is_off_until_the_project_key_exists() {
        assert!(PUBLIC_KEY.is_none() && !required(), "no real key may be committed by #32's visible part");
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
