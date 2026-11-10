//! Narrow elevation for installing/removing our game-local loader. Networking policy belongs to the OS.
use std::path::Path;
use anyhow::{Context, ensure};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PermissionResult { Applied, Cancelled }
/// Recognized before Iced initializes; the elevated helper never opens a manager or game window.
pub fn helper_mode() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--loader-helper")) { return None }
    let action = args.next(); let game = args.next();
    let result = match (action.as_deref().and_then(|v| v.to_str()), game) {
        (Some(action @ ("install" | "remove")), Some(game)) if args.next().is_none() => crate::direct::elevated_install(Path::new(&game), action == "remove"),
        _ => Err(anyhow::anyhow!("Expected install/remove and one game folder")),
    };
    Some(match result { Ok(()) => 0, Err(error) => { eprintln!("{error:#}"); 30 } })
}
pub fn loader_helper_arguments(game: &Path, remove: bool) -> anyhow::Result<String> {
    let path = game.to_str().context("Unsupported game path")?;
    ensure!(!path.contains(['"', '\r', '\n']) && !path.is_empty(), "Invalid game path");
    let trailing = path.chars().rev().take_while(|c| *c == '\\').count();
    Ok(format!("--loader-helper {} \"{}{}\"", if remove { "remove" } else { "install" }, path, "\\".repeat(trailing)))
}
pub fn request_loader(game: &Path, remove: bool) -> anyhow::Result<PermissionResult> {
    let action = if remove { "remove" } else { "install" };
    #[cfg(windows)] {
        let _ = action;
        elevated_windows(&loader_helper_arguments(game, remove)?)
    }
    #[cfg(not(windows))] {
        let output = std::process::Command::new("pkexec").arg(std::env::current_exe()?).args(["--loader-helper", action]).arg(game).output()?;
        if output.status.code() == Some(126) { return Ok(PermissionResult::Cancelled) }
        ensure!(output.status.success(), "Loader setup failed: {}", String::from_utf8_lossy(&output.stderr).trim());
        Ok(PermissionResult::Applied)
    }
}
#[cfg(windows)]
fn elevated_windows(arguments: &str) -> anyhow::Result<PermissionResult> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{Foundation::{CloseHandle, GetLastError, ERROR_CANCELLED},
        System::{Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE}, Threading::{WaitForSingleObject, GetExitCodeProcess, INFINITE}},
        UI::{Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_NOCLOSEPROCESS, SEE_MASK_NOASYNC, SEE_MASK_FLAG_NO_UI}, WindowsAndMessaging::SW_HIDE}};
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<_>>();
    let exe = wide(std::env::current_exe()?.as_os_str()); let verb = wide(std::ffi::OsStr::new("runas")); let args = wide(std::ffi::OsStr::new(arguments));
    unsafe {
        let com = CoInitializeEx(std::ptr::null(), (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32);
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed(); info.cbSize = std::mem::size_of_val(&info) as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
        info.lpVerb = verb.as_ptr(); info.lpFile = exe.as_ptr(); info.lpParameters = args.as_ptr(); info.nShow = SW_HIDE;
        let started = ShellExecuteExW(&mut info); let error = GetLastError();
        if com >= 0 { CoUninitialize(); }
        if started == 0 { if error == ERROR_CANCELLED { return Ok(PermissionResult::Cancelled) } anyhow::bail!("Windows could not start component setup (error {error})") }
        if info.hProcess.is_null() { return Ok(PermissionResult::Cancelled) }
        let waited = WaitForSingleObject(info.hProcess, INFINITE); let mut code = 30; let read = GetExitCodeProcess(info.hProcess, &mut code); CloseHandle(info.hProcess);
        ensure!(waited == 0 && read != 0 && code == 0, "Windows component setup was not completed (helper code {code}). Check permissions/policy, or check the game folder permissions.");
        Ok(PermissionResult::Applied)
    }
}
