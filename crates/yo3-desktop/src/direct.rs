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
        ensure!(previous.format == 1 && previous.executable == executable, "Cannot repair an unrecognized or changed loader installation");
        let old = regular(&receipt)?;
        std::fs::remove_file(&receipt)?;
        let result = install_files(game, source);
        if result.is_err() && !receipt.exists() { let _ = publish(&receipt, &old); }
        return result
    }
    if std::fs::symlink_metadata(&target).is_ok() || std::fs::symlink_metadata(&receipt).is_ok() {
        let previous = owned(game).context("An existing WINMM mod or unrecognized receipt was found; it will not be replaced")?;
        if previous.loader == loader { return Ok(()) }
        let old_dll = regular(&target)?; let old_receipt = regular(&receipt)?;
        remove_files(game)?;
        let result = install_files(game, source);
        if result.is_err() {
            if !receipt.exists() { let _ = publish(&receipt, &old_receipt); }
            if !target.exists() { let _ = publish(&target, &old_dll); }
        }
        return result
    }
    let body = serde_json::to_vec(&Receipt { format: 1, executable: executable.clone(), loader })?;
    // Publish with create_new: neither a foreign DLL nor a concurrently created receipt can be clobbered.
    publish(&receipt, &body)?;
    if let Err(error) = publish(&target, &bytes) { let _ = std::fs::remove_file(&receipt); return Err(error) }
    ensure!(hash(&regular(&game.join("yo3.exe"))?) == executable, "Game executable changed during installation");
    Ok(())
}
pub fn remove_files(game: &Path) -> anyhow::Result<()> {
    let receipt: Receipt = serde_json::from_slice(&regular(&game.join(RECEIPT))?)?;
    ensure!(receipt.format == 1 && receipt.executable == hash(&regular(&game.join("yo3.exe"))?), "The game or ownership receipt changed; removal refused");
    if std::fs::symlink_metadata(game.join("winmm.dll")).is_ok() { owned(game)?; std::fs::remove_file(game.join("winmm.dll"))?; }
    std::fs::remove_file(game.join(RECEIPT))?; Ok(())
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State { Disabled, Enabled, NeedsRepair(String), Conflict(String) }
#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct Stamp { game: String, prefix: String, paths: Vec<PathBuf>, hashes: Vec<String>, ini: String }
impl Backend {
    fn direct_helper(&self) -> anyhow::Result<PathBuf> {
        #[cfg(windows)] { Ok(self.runtime.launcher.clone()) }
        #[cfg(not(windows))] {
            let bundled = self.runtime.launcher.with_file_name("yo3-launcher-windows.exe");
            let manifest = bundled.with_file_name("bundle.json");
            if manifest.is_file() {
                let value: serde_json::Value = serde_json::from_slice(&std::fs::read(manifest)?)?;
                ensure!(value["components"]["direct_launcher"]["sha256"].as_str() == Some(hash(&regular(&bundled)?).as_str()), "The direct-launch helper differs from its release manifest. Restore the complete application bundle.");
                return Ok(bundled)
            }
            if bundled.is_file() { return Ok(bundled) }
            ensure!(yo3_tunnel::BUILD_VERSION.ends_with("-dev"), "The Windows direct-launch helper is missing. Restore the complete application bundle.");
            Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/x86_64-pc-windows-gnu/release/yo3-launcher.exe").canonicalize()?)
        }
    }
    fn direct_stamp(&self, settings: &Settings) -> anyhow::Result<Stamp> {
        let paths = vec![self.loader_source()?, self.runtime.dll.clone(), self.runtime.injector.clone(), self.direct_helper()?];
        let hashes = paths.iter().map(|p| regular(p).map(|bytes| hash(&bytes))).collect::<anyhow::Result<_>>()?;
        Ok(Stamp { game: settings.game_dir.clone(), prefix: settings.wine_prefix.clone(), paths, hashes, ini: hash(&regular(&self.root.join("profiles/loader.ini"))?) })
    }
    pub fn direct_state(&self, settings: &Settings) -> State {
        let game = Path::new(&settings.game_dir); let receipt = game.join(RECEIPT);
        if !receipt.exists() {
            if std::fs::symlink_metadata(game.join("winmm.dll")).is_ok() { return State::Conflict("An unowned WINMM mod is installed. Keep it intact; use Play here or resolve the mod conflict manually.".into()) }
            return if self.root.join("profiles/loader.ini").exists() { State::NeedsRepair("The game-local loader is missing.".into()) } else { State::Disabled }
        }
        if game.join("winmm.dll").exists() && let Err(error) = owned(game) { return State::Conflict(format!("{error:#}")) }
        let result = (|| -> anyhow::Result<()> {
            ensure!(game.join("winmm.dll").is_file(), "The owned loader is missing.");
            let saved: Stamp = serde_json::from_slice(&regular(&self.root.join("profiles/direct-setup.json"))?)?;
            ensure!(saved == self.direct_stamp(settings)?, "The game location or connection components changed.");
            yo3_launcher::Profile::load(&self.root.join("profiles/direct.json"))?; Ok(())
        })();
        match result { Ok(()) => State::Enabled, Err(error) => State::NeedsRepair(format!("{error:#}")) }
    }
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
        let helper = self.direct_helper()?;
        ensure!(helper.is_file(), "The Windows connection helper is missing. Extract the complete bundle.");
        self.check_game(settings)?;
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
        registry(settings, Some(&file))?;
        private_write(&state.join("direct-setup.json"), &serde_json::to_vec(&self.direct_stamp(settings)?)?, true)?;
        Ok(())
    }
    pub fn disable_direct(&self, settings: &Settings) -> anyhow::Result<()> {
        ensure!(!self.game_running(), "Close the game before changing direct-start setup");
        let game = Path::new(&settings.game_dir);
        if (game.join(RECEIPT).exists() || std::fs::symlink_metadata(game.join("winmm.dll")).is_ok()) && let Err(error) = remove_files(game) {
            if error.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied) {
                ensure!(crate::privileged::request_loader(Path::new(&settings.game_dir), true)? == crate::privileged::PermissionResult::Applied, "Direct-start removal cancelled");
            } else { return Err(error) }
        }
        registry(settings, None)?;
        for name in ["loader.ini", "direct.json", "direct-setup.json"] { let _ = std::fs::remove_file(self.root.join("profiles").join(name)); }
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
    fn a_packaged_direct_helper_must_exist_and_match_its_manifest() {
        use super::*;
        let root = std::env::temp_dir().join(format!("yo3-direct-helper-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut backend = Backend::new(root.clone()).unwrap();
        std::sync::Arc::get_mut(&mut backend).unwrap().runtime.launcher = root.join("yo3-launcher");
        let helper = root.join("yo3-launcher-windows.exe");
        std::fs::write(root.join("bundle.json"), serde_json::json!({"components":{"direct_launcher":{"sha256":hash(b"verified helper")}}}).to_string()).unwrap();
        assert!(backend.direct_helper().is_err(), "a bundle cannot borrow a helper from the developer checkout");
        std::fs::write(&helper, b"verified helper").unwrap();
        assert_eq!(backend.direct_helper().unwrap(), helper);
        std::fs::write(&helper, b"changed helper").unwrap();
        assert!(backend.direct_helper().is_err(), "direct start must reject mismatched helper code");
        drop(backend); std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn wine_batch_results_keep_unicode_paths_in_order_and_refuse_missing_or_extra_fields() {
        let text = "C:\\game folder\0Z:\\日本語\\mod.dll\0";
        assert_eq!(super::decode_windows_paths(text.as_bytes(), 2).unwrap(), ["C:\\game folder", "Z:\\日本語\\mod.dll"]);
        assert!(super::decode_windows_paths(text.as_bytes(), 1).is_err());
        assert!(super::decode_windows_paths(b"C:\\game\0", 2).is_err());
        assert!(super::decode_windows_paths(b"\xff\0", 1).is_err());
    }
}
