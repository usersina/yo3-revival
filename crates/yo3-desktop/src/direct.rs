//! Reversible, opt-in game-local loader. Credentials/configuration stay in the current user's private app data.
use std::path::{Path, PathBuf};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use crate::{Backend, Settings, SavedServer, private_write};
use yo3_launcher::components::hash;
const RECEIPT: &str = "yo3-revival-loader.json";
fn publish(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    let stage = path.with_extension(format!("{}.stage", std::process::id()));
    let mut file = yo3_tunnel::diagnostics::private_create(&stage)?;
    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?; file.sync_all()?;
        // These files contain public code/hashes. Use the owned handle, never chmod a replaceable path as root.
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; file.set_permissions(std::fs::Permissions::from_mode(0o644))?; }
        std::fs::hard_link(&stage, path)
    })();
    drop(file); let _ = std::fs::remove_file(stage);
    result.context("Cannot publish the loader without replacing an existing file")
}
fn regular(path: &Path) -> anyhow::Result<Vec<u8>> {
    let meta = std::fs::symlink_metadata(path)?;
    ensure!(meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= 32 * 1024 * 1024, "Refusing unexpected file: {}", path.display());
    Ok(std::fs::read(path)?)
}
#[derive(Serialize, Deserialize)]
struct Receipt { format: u32, executable: String, loader: String }
fn owned(game: &Path) -> anyhow::Result<Receipt> {
    let receipt: Receipt = serde_json::from_slice(&regular(&game.join(RECEIPT))?)?;
    ensure!(receipt.format == 1 && receipt.executable == hash(&regular(&game.join("yo3.exe"))?)
        && receipt.loader == hash(&regular(&game.join("winmm.dll"))?), "Game or loader changed since installation. Refusing to overwrite/remove another mod.");
    Ok(receipt)
}
/// Source is selected by the caller from the application's validated bundle, never from elevated command arguments.
pub fn install_files(game: &Path, source: &Path) -> anyhow::Result<()> {
    let executable = hash(&regular(&game.join("yo3.exe"))?); let bytes = regular(source)?; let loader = hash(&bytes);
    let target = game.join("winmm.dll"); let receipt = game.join(RECEIPT);
    if std::fs::symlink_metadata(&target).is_err() && std::fs::symlink_metadata(&receipt).is_ok() {
        let previous: Receipt = serde_json::from_slice(&regular(&receipt)?)?;
        ensure!(previous.format == 1 && previous.executable == executable && previous.loader == loader, "Cannot repair an unrecognized or changed loader installation");
        return publish(&target, &bytes)
    }
    if std::fs::symlink_metadata(&target).is_ok() || std::fs::symlink_metadata(&receipt).is_ok() {
        ensure!(owned(game).context("An existing WINMM mod or unrecognized receipt was found; it will not be replaced")?.loader == loader, "An older owned loader is installed. Disable direct start before installing this release."); return Ok(())
    }
    let body = serde_json::to_vec(&Receipt { format: 1, executable: executable.clone(), loader })?;
    // Publish with create_new: neither a foreign DLL nor a concurrently created receipt can be clobbered.
    publish(&receipt, &body)?;
    if let Err(error) = publish(&target, &bytes) { let _ = std::fs::remove_file(&receipt); return Err(error) }
    ensure!(hash(&regular(&game.join("yo3.exe"))?) == executable, "Game executable changed during installation");
    Ok(())
}
pub fn remove_files(game: &Path) -> anyhow::Result<()> {
    owned(game)?; std::fs::remove_file(game.join("winmm.dll"))?; std::fs::remove_file(game.join(RECEIPT))?; Ok(())
}
#[cfg(not(windows))]
fn wine_command(settings: &Settings, program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    if !settings.wine_prefix.is_empty() { cmd.env("WINEPREFIX", &settings.wine_prefix); }
    cmd.env("WINEDEBUG", "-all"); cmd
}
fn windows_path(path: &Path, settings: &Settings) -> anyhow::Result<String> {
    Ok(windows_paths(&[path], settings)?.remove(0))
}
#[cfg(not(windows))]
fn decode_windows_paths(bytes: &[u8], count: usize) -> anyhow::Result<Vec<String>> {
    ensure!(bytes.last() == Some(&0), "Wine returned an incomplete path conversion");
    let paths: Vec<String> = std::str::from_utf8(bytes)?.split_terminator('\0').map(str::to_owned).collect();
    ensure!(paths.len() == count && paths.iter().all(|p| !p.is_empty() && !p.contains(['\r', '\n'])), "Wine returned an unexpected path conversion");
    Ok(paths)
}
fn windows_paths(paths: &[&Path], settings: &Settings) -> anyhow::Result<Vec<String>> {
    #[cfg(windows)] { let _ = settings; Ok(paths.iter().map(|p| p.to_string_lossy().into_owned()).collect()) }
    #[cfg(not(windows))] {
        let result = wine_command(settings, "winepath").args(["-0", "-w"]).args(paths).output()?;
        ensure!(result.status.success(), "Cannot translate the path into the selected Wine prefix");
        decode_windows_paths(&result.stdout, paths.len())
    }
}
fn registry(settings: &Settings, config: Option<&Path>) -> anyhow::Result<()> {
    #[cfg(windows)] let mut cmd = std::process::Command::new("reg.exe");
    #[cfg(not(windows))] let mut cmd = { let mut c = wine_command(settings, "wine"); c.arg("reg.exe"); c };
    let key = "HKCU\\Software\\YO3 Revival";
    if let Some(config) = config { cmd.args(["add", key, "/v", "LoaderConfig", "/t", "REG_SZ", "/d", &windows_path(config, settings)?, "/f"]); }
    else { cmd.args(["delete", key, "/v", "LoaderConfig", "/f"]); }
    crate::hide_console(&mut cmd);
    ensure!(cmd.output()?.status.success(), "Cannot update direct-start configuration for this user");
    #[cfg(not(windows))] if config.is_some() {
        ensure!(wine_command(settings, "wine").args(["reg.exe", "add", "HKCU\\Software\\Wine\\AppDefaults\\yo3.exe\\DllOverrides", "/v", "winmm", "/d", "native,builtin", "/f"]).output()?.status.success(), "Cannot enable the game-local loader in this Wine prefix");
    }
    Ok(())
}
impl Backend {
    pub fn loader_source(&self) -> anyhow::Result<PathBuf> {
        let bundle = self.runtime.injector.parent().context("Application folder missing")?;
        let source = if bundle.join("yo3-bootstrap.dll").is_file() { bundle.join("yo3-bootstrap.dll") } else { bundle.join("winmm.dll") };
        yo3_launcher::components::inspect_mod(&source).context("The automatic loader is missing or incompatible; extract the complete application bundle")?;
        if bundle.join("bundle.json").is_file() {
            let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(bundle.join("bundle.json"))?)?;
            ensure!(manifest["components"]["loader"]["sha256"].as_str() == Some(hash(&regular(&source)?).as_str()), "Loader differs from the release manifest");
        }
        Ok(source)
    }
    pub fn direct_profile(&self, settings: &Settings, remote: Option<SavedServer>) -> anyhow::Result<()> {
        let file = self.root.join("profiles/direct.json");
        // Clear stale credentials first; a failed selection change must never reconnect to the previous account.
        if file.exists() { std::fs::remove_file(&file)?; }
        ensure!(settings.selected_server.is_none() || remote.is_some(), "The saved connection is missing; select a server/account and enable direct start again");
        let server = match remote { Some(s) => s, None => self.local_connection(settings, &settings.local_account)? };
        server.validate()?;
        let identity = server.identity.clone().unwrap_or(yo3_tunnel::identity_from_pem(server.certificate.as_bytes())?);
        private_write(&file, serde_json::json!({"server":server.address,"server_name":server.server_name,"identity":identity,"key":server.key}).to_string().as_bytes(), true)?;
        Ok(())
    }
    pub fn sync_direct(&self, settings: &Settings, remote: Option<SavedServer>) -> anyhow::Result<()> {
        if self.root.join("profiles/loader.ini").is_file() { self.direct_profile(settings, remote)?; }
        Ok(())
    }
    pub fn enable_direct(&self, settings: &Settings, remote: Option<SavedServer>) -> anyhow::Result<()> {
        ensure!(!self.game_running(), "Close the game before changing direct-start setup");
        let game = Path::new(&settings.game_dir); crate::validate_game(game)?;
        yo3_launcher::components::preflight(&game.join("yo3.exe"), &self.runtime.injector, &self.runtime.dll)?;
        let source = self.loader_source()?;
        #[cfg(windows)] let helper = self.runtime.launcher.clone();
        #[cfg(not(windows))] let helper = {
            let bundled = self.runtime.launcher.with_file_name("yo3-launcher-windows.exe");
            if bundled.is_file() { bundled } else { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/x86_64-pc-windows-gnu/release/yo3-launcher.exe").canonicalize()? }
        };
        ensure!(helper.is_file(), "The Windows connection helper is missing. Extract the complete bundle.");
        self.direct_profile(settings, remote)?;
        let state = self.root.join("profiles"); let file = state.join("loader.ini");
        let mut ini = String::from("[revival]\r\n");
        let profile = state.join("direct.json"); let logs = self.root.join("logs/sessions");
        let values = windows_paths(&[&helper, &self.runtime.injector, &self.runtime.dll, &profile, &state, &logs], settings)?;
        for (key, value) in ["launcher", "injector", "dll", "profile", "state", "logs"].into_iter().zip(values) {
            ensure!(!value.contains(['\r', '\n']), "Invalid setup path"); ini.push_str(&format!("{key}={value}\r\n"));
        }
        ini.push_str(&format!("sha256={}\r\n", hash(&regular(&self.runtime.dll)?)));
        let bytes: Vec<u8> = std::iter::once(0xfeffu16).chain(ini.encode_utf16()).flat_map(u16::to_le_bytes).collect();
        private_write(&file, &bytes, true)?;
        if let Err(error) = install_files(game, &source) {
            if error.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied) {
                ensure!(crate::privileged::request_loader(game, false)? == crate::privileged::PermissionResult::Applied, "Direct-start installation cancelled");
            } else { return Err(error) }
        }
        registry(settings, Some(&file))?; Ok(())
    }
    pub fn disable_direct(&self, settings: &Settings) -> anyhow::Result<()> {
        ensure!(!self.game_running(), "Close the game before changing direct-start setup");
        if let Err(error) = remove_files(Path::new(&settings.game_dir)) {
            if error.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied) {
                ensure!(crate::privileged::request_loader(Path::new(&settings.game_dir), true)? == crate::privileged::PermissionResult::Applied, "Direct-start removal cancelled");
            } else { return Err(error) }
        }
        registry(settings, None)?;
        for name in ["loader.ini", "direct.json"] { let _ = std::fs::remove_file(self.root.join("profiles").join(name)); }
        Ok(())
    }
}

pub fn elevated_install(game: &Path, remove: bool) -> anyhow::Result<()> {
    if remove { return remove_files(game) }
    crate::validate_game(game)?;
    let runtime = crate::Runtime::discover()?; let bin = runtime.injector.parent().context("Missing bundle")?;
    let source = if bin.join("yo3-bootstrap.dll").is_file() { bin.join("yo3-bootstrap.dll") } else { bin.join("winmm.dll") };
    yo3_launcher::components::inspect_mod(&source)?;
    if bin.join("bundle.json").is_file() {
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(bin.join("bundle.json"))?)?;
        ensure!(manifest["components"]["loader"]["sha256"].as_str() == Some(hash(&regular(&source)?).as_str()), "Bundled loader integrity check failed");
    }
    install_files(game, &source)
}

#[cfg(all(test, not(windows)))]
mod tests {
    #[test]
    fn wine_batch_results_keep_unicode_paths_in_order_and_refuse_missing_or_extra_fields() {
        let text = "C:\\game folder\0Z:\\日本語\\mod.dll\0";
        assert_eq!(super::decode_windows_paths(text.as_bytes(), 2).unwrap(), ["C:\\game folder", "Z:\\日本語\\mod.dll"]);
        assert!(super::decode_windows_paths(text.as_bytes(), 1).is_err());
        assert!(super::decode_windows_paths(b"C:\\game\0", 2).is_err());
        assert!(super::decode_windows_paths(b"\xff\0", 1).is_err());
    }
}
