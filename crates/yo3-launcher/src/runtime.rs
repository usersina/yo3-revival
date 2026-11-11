//! A separate 32-bit helper probes the game's legacy runtimes without executing the game.
use std::path::Path;
use std::process::Stdio;
use anyhow::Context;

#[cfg(not(windows))]
pub async fn windows_path(path: &Path, prefix: Option<&Path>) -> anyhow::Result<String> {
    let mut cmd = tokio::process::Command::new("winepath");
    cmd.arg("-w").arg(path).stderr(Stdio::null()).kill_on_drop(true);
    if let Some(prefix) = prefix { cmd.env("WINEPREFIX", prefix); }
    let out = tokio::time::timeout(std::time::Duration::from_secs(30), cmd.output()).await
        .context("Wine path setup timed out. Close the game, check that Wine is installed and its prefix is writable, then try again")?
        .context("Wine path setup could not start. Install Wine, then validate again")?;
    anyhow::ensure!(out.status.success(), "Wine path setup failed. Check that Wine is installed and its prefix is writable");
    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}

pub async fn check(game: &Path, injector: &Path, prefix: Option<&Path>) -> anyhow::Result<()> {
    let game = game.canonicalize()?; let injector = injector.canonicalize()?;
    #[cfg(windows)]
    let mut cmd = { let _ = prefix; let mut cmd = tokio::process::Command::new(injector); cmd.arg("--check-runtime").arg(game); cmd };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut cmd = tokio::process::Command::new("wine");
        cmd.arg(injector).arg("--check-runtime").arg(windows_path(&game, prefix).await?);
        if let Some(prefix) = prefix { cmd.env("WINEPREFIX", prefix); }
        cmd
    };
    #[cfg(windows)] cmd.creation_flags(0x08000000); // Keep the console-only checker out of the desktop flow.
    cmd.stdin(Stdio::null()).kill_on_drop(true);
    let out = tokio::time::timeout(std::time::Duration::from_secs(45), cmd.output()).await
        .context("Game runtime validation timed out. Close any stuck game/Wine processes, then validate again. Export logs if this repeats")?
        .context("Cannot start the runtime checker (yo3-auth-launch.exe). Restore the complete application bundle; check security-software detection history if the file disappeared")?;
    if !out.status.success() {
        // Diagnostics are bounded before being shown; no credentials are supplied to this probe.
        let mut tail = &out.stderr[out.stderr.len().saturating_sub(8192)..];
        if out.stderr.len() > 8192 { tail = &tail[tail.iter().position(|b| *b == b'\n').map_or(tail.len(), |n| n+1)..]; }
        let detail = yo3_tunnel::diagnostics::redact(&String::from_utf8_lossy(tail), &[], None);
        let summary = if detail.contains("d3dx9_40.dll") {
            "The game needs its legacy DirectX runtime. Install DirectX End-User Runtimes (June 2010), then validate again."
        } else if detail.contains("MSVCR90.dll") || detail.contains("MSVCP90.dll") || detail.contains("stage=runtime-manifest") {
            "The game could not load Microsoft Visual C++ 2008 (x86). Install or repair it, then validate again."
        } else { "Game runtime validation failed. Check the details or export logs for help." };
        anyhow::bail!("{summary}\nRuntime checker ({}): {}", out.status, if detail.trim().is_empty() {
            "The runtime checker exited without details. Check security-software detection history and export logs for help."
        } else { detail.trim() });
    }
    Ok(())
}
