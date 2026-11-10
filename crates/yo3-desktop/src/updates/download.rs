use std::path::PathBuf;
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering}};
use std::time::Duration;
use anyhow::{ensure, Context};
use super::{Release, install};
#[derive(Clone, Default)]
pub struct Transfer(Arc<Progress>);
#[derive(Default)]
struct Progress { cancelled: AtomicBool, bytes: AtomicU64, phase: AtomicU8 }
impl Transfer {
    pub fn cancel(&self) { self.0.cancelled.store(true, Ordering::Relaxed); }
    pub fn cancelled(&self) -> bool { self.0.cancelled.load(Ordering::Relaxed) }
    pub fn bytes(&self) -> u64 { self.0.bytes.load(Ordering::Relaxed) }
    pub fn verifying(&self) -> bool { self.0.phase.load(Ordering::Relaxed) == 1 }
    async fn wait<T>(&self, future: impl std::future::Future<Output=T>) -> anyhow::Result<T> {
        tokio::pin!(future);
        loop { ensure!(!self.cancelled(), "Update cancelled"); tokio::select! {
            value = &mut future => return Ok(value), _ = tokio::time::sleep(Duration::from_millis(100)) => {},
        } }
    }
}
fn allowed(url: &reqwest::Url) -> bool {
    url.scheme() == "https" && url.username().is_empty() && url.password().is_none() && url.port_or_known_default() == Some(443)
        && matches!(url.host_str(), Some("github.com" | "release-assets.githubusercontent.com" | "objects.githubusercontent.com"))
}
async fn response(client: &reqwest::Client, release: &Release, transfer: &Transfer) -> anyhow::Result<reqwest::Response> {
    let mut url = reqwest::Url::parse(&release.asset.url)?;
    for _ in 0..6 {
        ensure!(allowed(&url), "Update redirected outside GitHub's release service");
        let result = transfer.wait(client.get(url.clone()).send()).await??;
        if result.status().is_redirection() {
            let location = result.headers().get(reqwest::header::LOCATION).context("Update redirect has no destination")?.to_str()?;
            url = url.join(location)?;
        } else { ensure!(result.status().is_success(), "Update download failed (HTTP {})", result.status().as_u16()); return Ok(result) }
    }
    anyhow::bail!("Update download redirected too many times")
}
async fn save(mut response: reqwest::Response, path: &std::path::Path, release: &Release, transfer: &Transfer) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    ensure!(release.asset.bytes > 0 && release.asset.bytes <= super::MAX_DOWNLOAD && response.content_length().is_none_or(|n| n == release.asset.bytes), "Update download length does not match GitHub");
    let mut file = tokio::fs::File::from_std(yo3_tunnel::diagnostics::private_create(path)?);
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256); let mut received = 0;
    while let Some(chunk) = transfer.wait(response.chunk()).await?? {
        ensure!(chunk.len() as u64 <= release.asset.bytes.saturating_sub(received), "Update download exceeds its declared size");
        file.write_all(&chunk).await?; hash.update(&chunk); received += chunk.len() as u64;
        transfer.0.bytes.store(received, Ordering::Relaxed);
    }
    ensure!(!transfer.cancelled(), "Update cancelled"); file.sync_all().await?;
    let digest: String = hash.finish().as_ref().iter().map(|b| format!("{b:02x}")).collect();
    ensure!(received == release.asset.bytes && digest == release.asset.sha256, "Update download checksum or length does not match GitHub");
    Ok(())
}
pub async fn prepare(root: PathBuf, release: Release, transfer: Transfer) -> anyhow::Result<install::Installed> {
    install::platform(&release)?;
    ensure!(Some(install::platform(&release)?) == super::Platform::current().name(), "Update is for another platform");
    let job = install::job_directory(&root)?; let archive = job.0.join("download");
    let client = super::client(Duration::from_secs(15 * 60))?;
    let reply = response(&client, &release, &transfer).await?;
    save(reply, &archive, &release, &transfer).await?; transfer.0.phase.store(1, Ordering::Relaxed);
    tokio::task::spawn_blocking(move || { let _job = job; install::stage_with(&root, &release, &archive, &|| transfer.cancelled()) }).await?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_redirects_cannot_send_downloads_to_unrelated_hosts_or_credentials() {
        for url in ["http://github.com/a", "https://github.com.evil.test/a", "https://user@github.com/a", "https://github.com:444/a", "https://127.0.0.1/a"] { assert!(!allowed(&url.parse().unwrap())); }
        assert!(allowed(&"https://release-assets.githubusercontent.com/asset?signature=value".parse().unwrap()));
    }
    #[tokio::test]
    async fn cancellation_interrupts_a_stalled_request() {
        let transfer = Transfer::default(); let other = transfer.clone();
        let task = tokio::spawn(async move { other.wait(std::future::pending::<()>()).await });
        transfer.cancel(); assert!(tokio::time::timeout(Duration::from_secs(1), task).await.unwrap().unwrap().is_err());
    }
    #[tokio::test]
    async fn streamed_downloads_are_bounded_and_verified() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let _ = yo3_tunnel::tokio_rustls::rustls::crypto::ring::default_provider().install_default();
        for (body, expected) in [("fixture", "fixture"), ("changed", "fixture"), ("too long", "fixture")] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap(); let address = listener.local_addr().unwrap();
            let task = tokio::spawn(async move { let (mut socket,_) = listener.accept().await.unwrap(); let mut request=[0;4096]; let _=socket.read(&mut request).await; socket.write_all(format!("HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n{body}").as_bytes()).await.unwrap(); });
            let root=std::env::temp_dir().join(format!("yo3-download-{}-{address}",std::process::id()).replace(':',"-")); std::fs::create_dir_all(&root).unwrap();
            let release=Release{tag:String::new(),url:String::new(),notes:String::new(),asset:super::super::Asset{name:String::new(),url:String::new(),bytes:expected.len() as u64,sha256:yo3_launcher::components::hash(expected.as_bytes())}};
            let response=reqwest::Client::new().get(format!("http://{address}")).send().await.unwrap();
            assert_eq!(save(response,&root.join("download"),&release,&Transfer::default()).await.is_ok(),body == expected);
            task.await.unwrap(); std::fs::remove_dir_all(root).unwrap();
        }
    }
}
