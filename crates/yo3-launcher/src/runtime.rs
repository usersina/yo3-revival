//! A separate 32-bit helper probes the game's legacy runtimes without executing the game.
use std::path::Path;
use std::process::Stdio;
use anyhow::Context;

/// The Wine program to use instead of `wine` on the PATH (the launcher's Settings → Advanced): its folder's
/// `winepath` and the rest come with it.
pub const WINE_ENV: &str = "YO3_WINE";

/// The Wine tool `name` ("wine", "winepath"…): beside the chosen Wine program `wine`, else on the PATH.
pub fn wine_tool_of(wine: Option<&std::ffi::OsStr>, name: &str) -> std::ffi::OsString {
    let Some(wine) = wine.filter(|w| !w.is_empty()) else { return name.into() };
    let wine = Path::new(wine);
    if name == "wine" { return wine.as_os_str().to_owned() }
    let beside = wine.with_file_name(format!("{name}{}", wine.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default()));
    if beside.is_file() { beside.into_os_string() } else { name.into() }
}

/// [`wine_tool_of`] for the Wine program this process was given in [`WINE_ENV`].
pub fn wine_tool(name: &str) -> std::ffi::OsString { wine_tool_of(std::env::var_os(WINE_ENV).as_deref(), name) }

/// Absolute paths passed to the original game's Win32 image loader.
pub fn native_path(path: &Path) -> std::io::Result<std::path::PathBuf> {
    // The native image loader rejects ordinary files when passed std's Windows verbatim prefix.
    // Keep extended paths only when simplifying them would select a different file.
    #[cfg(windows)] { dunce::canonicalize(path) }
    #[cfg(not(windows))] { path.canonicalize() }
}

#[cfg(not(windows))]
pub async fn windows_path(path: &Path, prefix: Option<&Path>) -> anyhow::Result<String> {
    let mut cmd = tokio::process::Command::new(wine_tool("winepath"));
    cmd.arg("-w").arg(path).stderr(Stdio::null()).kill_on_drop(true);
    if let Some(prefix) = prefix { cmd.env("WINEPREFIX", prefix); }
    // A first start of a Wine prefix can take most of a minute on a busy PC. The timeout's own words ("deadline has elapsed")
    // are left out: the sentence says what happened.
    let out = tokio::time::timeout(std::time::Duration::from_secs(60), cmd.output()).await
        .map_err(|_| anyhow::anyhow!("Wine took too long to start. Close the game and any stuck Wine windows, then retry the check"))?
        .context("Wine path setup could not start. Install Wine, then retry the check")?;
    anyhow::ensure!(out.status.success(), "Wine path setup failed. Check that Wine is installed and its prefix is writable");
    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}

pub async fn check(game: &Path, injector: &Path, prefix: Option<&Path>) -> anyhow::Result<()> {
    let game = native_path(game)?; let injector = native_path(injector)?;
    #[cfg(windows)]
    let mut cmd = { let _ = prefix; let mut cmd = tokio::process::Command::new(injector); cmd.arg("--check-runtime").arg(game); cmd };
    #[cfg(not(windows))]
    let mut cmd = {
        let mut cmd = tokio::process::Command::new(wine_tool("wine"));
        cmd.arg(injector).arg("--check-runtime").arg(windows_path(&game, prefix).await?);
        if let Some(prefix) = prefix { cmd.env("WINEPREFIX", prefix); }
        cmd
    };
    #[cfg(windows)] cmd.creation_flags(0x08000000); // Keep the console-only checker out of the desktop flow.
    cmd.stdin(Stdio::null()).kill_on_drop(true);
    let out = tokio::time::timeout(std::time::Duration::from_secs(45), cmd.output()).await
        .map_err(|_| anyhow::anyhow!("The game's check took too long. Close any stuck game or Wine windows, then retry the check. Export logs if this repeats"))?
        .context("Cannot start the runtime checker (yo3-auth-launch.exe). Restore the complete application bundle; check security-software detection history if the file disappeared")?;
    if !out.status.success() {
        // Diagnostics are bounded before being shown; no credentials are supplied to this probe.
        let mut tail = &out.stderr[out.stderr.len().saturating_sub(8192)..];
        if out.stderr.len() > 8192 { tail = &tail[tail.iter().position(|b| *b == b'\n').map_or(tail.len(), |n| n+1)..]; }
        let detail = yo3_tunnel::diagnostics::redact(&String::from_utf8_lossy(tail), &[], None);
        let summary = if detail.contains("d3dx9_40.dll") {
            "The game needs its legacy DirectX runtime. Install DirectX End-User Runtimes (June 2010), then retry the check."
        } else if detail.contains("MSVCR90.dll") || detail.contains("MSVCP90.dll") || detail.contains("stage=runtime-manifest") {
            "The game could not load Microsoft Visual C++ 2008 (x86). Install or repair it, then retry the check."
        } else { "Game runtime check failed. Check the details or export logs for help." };
        anyhow::bail!("{summary}\nRuntime checker ({}): {}", out.status, if detail.trim().is_empty() {
            "The runtime checker exited without details. Check security-software detection history and export logs for help."
        } else { detail.trim() });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_chosen_wine_brings_its_own_tools() {
        let dir = std::env::temp_dir().join(format!("yo3-wine-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let wine = dir.join("wine");
        assert_eq!(wine_tool_of(None, "winepath"), "winepath");
        assert_eq!(wine_tool_of(Some(wine.as_os_str()), "wine"), wine.as_os_str());
        assert_eq!(wine_tool_of(Some(wine.as_os_str()), "winepath"), "winepath", "none beside it: the PATH's");
        std::fs::write(dir.join("winepath"), b"").unwrap();
        assert_eq!(wine_tool_of(Some(wine.as_os_str()), "winepath"), dir.join("winepath").as_os_str());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
