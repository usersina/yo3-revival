//! Read-only, bounded release discovery. Application credentials are never part of an update request.
use anyhow::{Context, ensure};
use serde::Deserialize;
use semver::Version;
use std::time::Duration;
pub mod install;
pub mod activation;
pub mod download;
pub const REPOSITORY: &str = "https://github.com/usersina/yo3-revival";
pub const RELEASES: &str = "https://github.com/usersina/yo3-revival/releases";
const LATEST: &str = "https://api.github.com/repos/usersina/yo3-revival/releases/latest";
const MAX_METADATA: usize = 1024 * 1024;
pub const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;
#[derive(Clone, Copy)]
pub enum Platform { LinuxX64, WindowsX64, Unsupported }
impl Platform {
    pub fn name(self) -> Option<&'static str> { match self { Self::LinuxX64 => Some("linux-x86_64"), Self::WindowsX64 => Some("windows-x86_64"), Self::Unsupported => None } }
    pub fn current() -> Self {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => Self::LinuxX64, ("windows", "x86_64") => Self::WindowsX64, _ => Self::Unsupported,
        }
    }
    fn suffix(self) -> Option<&'static str> { match self { Self::LinuxX64 => Some("linux-x86_64.tar.gz"), Self::WindowsX64 => Some("windows-x86_64.zip"), Self::Unsupported => None } }
}
#[derive(Clone, Debug)]
pub struct Asset { pub name: String, pub url: String, pub bytes: u64, pub sha256: String }
#[derive(Clone, Debug)]
pub struct Release { pub tag: String, pub url: String, pub notes: String, pub asset: Asset }
#[derive(Clone, Debug)]
pub enum Decision { Current { latest: String }, Available(Release), Development { latest: String }, Manual { latest: String }, NoReleases }
#[derive(Deserialize)]
struct Metadata { tag_name: String, #[serde(default)] draft: bool, #[serde(default)] prerelease: bool,
    #[serde(default)] body: Option<String>, #[serde(default)] assets: Vec<MetadataAsset> }
#[derive(Deserialize)]
struct MetadataAsset { name: String, size: u64, digest: Option<String> }
fn version(text: &str) -> anyhow::Result<Version> { Ok(Version::parse(text.strip_prefix('v').unwrap_or(text))?) }
pub fn inspect(current: &str, platform: Platform, body: &[u8]) -> anyhow::Result<Decision> {
    ensure!(body.len() <= MAX_METADATA, "GitHub returned oversized release metadata");
    let metadata: Metadata = serde_json::from_slice(body).context("GitHub returned invalid release metadata")?;
    let candidate = version(&metadata.tag_name).context("Unsupported release version")?;
    ensure!(!metadata.draft && !metadata.prerelease && candidate.pre.is_empty(), "The latest release is not a published stable release");
    let tag = metadata.tag_name;
    let Ok(installed) = version(current) else { return Ok(Decision::Development { latest: tag }) };
    if !installed.pre.is_empty() { return Ok(Decision::Development { latest: tag }) }
    if candidate.cmp_precedence(&installed).is_le() { return Ok(Decision::Current { latest: tag }) }
    let Some(suffix) = platform.suffix() else { return Ok(Decision::Manual { latest: tag }) };
    let name = format!("yo3-revival-{tag}-{suffix}");
    let matches: Vec<_> = metadata.assets.into_iter().filter(|a| a.name == name).collect();
    if matches.is_empty() { return Ok(Decision::Manual { latest: tag }) }
    ensure!(matches.len() == 1, "GitHub returned ambiguous update assets");
    let asset = &matches[0]; ensure!(asset.size > 0 && asset.size <= MAX_DOWNLOAD, "Release download size is unsupported");
    let digest = asset.digest.as_deref().and_then(|d| d.strip_prefix("sha256:")).context("Release download has no SHA-256 digest; use GitHub to review it")?;
    ensure!(digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()), "Release download has an invalid SHA-256 digest");
    // Construct URLs from a validated semantic tag and our fixed repository, never from response-supplied URLs.
    let url = format!("{REPOSITORY}/releases/tag/{tag}");
    let asset = Asset { name: name.clone(), url: format!("{REPOSITORY}/releases/download/{tag}/{name}"), bytes: asset.size, sha256: digest.to_ascii_lowercase() };
    let notes: String = metadata.body.unwrap_or_default().chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').take(20_000).collect();
    Ok(Decision::Available(Release { tag, url, notes, asset }))
}
fn client(timeout: Duration) -> anyhow::Result<reqwest::Client> {
    let _ = yo3_tunnel::tokio_rustls::rustls::crypto::ring::default_provider().install_default();
    Ok(reqwest::Client::builder().https_only(true).redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(4)).read_timeout(Duration::from_secs(30)).timeout(timeout)
        .user_agent(format!("yo3-revival/{}", yo3_tunnel::BUILD_VERSION)).build()?)
}
pub async fn check() -> anyhow::Result<Decision> {
    fetch(&client(Duration::from_secs(10))?, LATEST, yo3_tunnel::BUILD_VERSION, Platform::current()).await
}
async fn fetch(client: &reqwest::Client, url: &str, current: &str, platform: Platform) -> anyhow::Result<Decision> {
    let mut response = client.get(url).header("Accept", "application/vnd.github+json").header("X-GitHub-Api-Version", "2026-03-10")
        .send().await.context("Could not reach GitHub; you can keep playing and retry later")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND { return Ok(Decision::NoReleases) }
    ensure!(!matches!(response.status().as_u16(), 403 | 429), "GitHub limited update requests; retry later");
    ensure!(response.status().is_success(), "GitHub update check failed (HTTP {})", response.status().as_u16());
    ensure!(response.content_length().unwrap_or(0) <= MAX_METADATA as u64, "GitHub returned oversized release metadata");
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? { ensure!(body.len()+chunk.len() <= MAX_METADATA, "GitHub returned oversized release metadata"); body.extend_from_slice(&chunk); }
    inspect(current, platform, &body)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn release_checks_bound_offline_rate_limited_slow_and_oversized_responses() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let _ = yo3_tunnel::tokio_rustls::rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder().timeout(Duration::from_millis(200)).build().unwrap();
        for (reply, slow, expected) in [("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n".to_string(),false,"none"),
            ("HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\n\r\n".into(),false,"limited"),
            (format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",MAX_METADATA+1),false,"oversized"),
            ("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\n".into(),true,"error")] {
            let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(); let address=listener.local_addr().unwrap();
            let server=tokio::spawn(async move { let (mut socket,_)=listener.accept().await.unwrap(); let mut request=[0;4096]; let n=socket.read(&mut request).await.unwrap();
                assert!(!String::from_utf8_lossy(&request[..n]).to_ascii_lowercase().contains("authorization:"));
                socket.write_all(reply.as_bytes()).await.unwrap(); if slow { tokio::time::sleep(Duration::from_secs(1)).await; } });
            let result=tokio::time::timeout(Duration::from_secs(2),fetch(&client,&format!("http://{address}"),"0.1.0",Platform::LinuxX64)).await.unwrap();
            match expected { "none" => assert!(matches!(result.unwrap(),Decision::NoReleases)), "error" => assert!(result.is_err()), text => assert!(result.unwrap_err().to_string().contains(text)) }
            server.abort();
        }
        let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(); let address=listener.local_addr().unwrap(); drop(listener);
        assert!(fetch(&client,&format!("http://{address}"),"0.1.0",Platform::LinuxX64).await.is_err());
    }
}
