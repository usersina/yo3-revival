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
    let name = args.next();
    let result = match (action.as_deref().and_then(|v| v.to_str()), game, name) {
        (Some(action @ ("install" | "remove")), Some(game), None) if args.next().is_none() => crate::direct::elevated_install(Path::new(&game), action == "remove"),
        // Move it aside / Put back (#68) in a protected game folder: one plugin file, checked again here.
        (Some(action @ ("aside" | "back")), Some(game), Some(name)) if args.next().is_none() => name.to_str().context("Unsupported file name").and_then(|name| {
            if action == "aside" { crate::foreign::move_aside(Path::new(&game), name) } else { crate::foreign::put_back(Path::new(&game), name) }.map(|_| ())
        }),
        _ => Err(anyhow::anyhow!("Expected install/remove and one game folder, or aside/back, a game folder and a file name")),
    };
    Some(match result { Ok(()) => 0, Err(error) => { eprintln!("{error:#}"); 30 } })
}
pub fn loader_helper_arguments(game: &Path, remove: bool) -> anyhow::Result<String> {
    let path = game.to_str().context("Unsupported game path")?;
    ensure!(!path.contains(['"', '\r', '\n']) && !path.is_empty(), "Invalid game path");
    let trailing = path.chars().rev().take_while(|c| *c == '\\').count();
    Ok(format!("--loader-helper {} \"{}{}\"", if remove { "remove" } else { "install" }, path, "\\".repeat(trailing)))
}
/// The arguments that ask the elevated helper to move `name` in `game` aside, or put it back.
pub fn file_helper_arguments(game: &Path, name: &str, back: bool) -> anyhow::Result<String> {
    ensure!(!name.is_empty() && !name.contains(['"', '\r', '\n', '/', '\\']), "Invalid file name");
    let path = loader_helper_arguments(game, false)?;
    let game = path.strip_prefix("--loader-helper install ").context("Invalid game path")?;
    Ok(format!("--loader-helper {} {game} \"{name}\"", if back { "back" } else { "aside" }))
}
/// Asks Windows (or polkit) for permission to rename one file in a protected game folder (#68), as the loader's Allow does.
pub fn request_file(game: &Path, name: &str, back: bool) -> anyhow::Result<PermissionResult> {
    #[cfg(windows)] { elevated_windows(&file_helper_arguments(game, name, back)?) }
    #[cfg(not(windows))] {
        let output = std::process::Command::new("pkexec").arg(std::env::current_exe()?).args(["--loader-helper", if back { "back" } else { "aside" }]).arg(game).arg(name).output()?;
        if output.status.code() == Some(126) { return Ok(PermissionResult::Cancelled) }
        ensure!(output.status.success(), "File setup failed: {}", String::from_utf8_lossy(&output.stderr).trim());
        Ok(PermissionResult::Applied)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_elevated_file_helper_gets_one_quoted_game_folder_and_one_plain_file_name() {
        let game = Path::new("C:\\Program Files (x86)\\Konami\\Yu-Gi-Oh! ONLINE 3");
        assert_eq!(file_helper_arguments(game, "dinput8.dll", false).unwrap(), "--loader-helper aside \"C:\\Program Files (x86)\\Konami\\Yu-Gi-Oh! ONLINE 3\" \"dinput8.dll\"");
        assert!(file_helper_arguments(game, "dinput8.dll", true).unwrap().starts_with("--loader-helper back "));
        for bad in ["", "a\"b.dll", "..\\x.dll", "sub/x.dll"] { assert!(file_helper_arguments(game, bad, false).is_err(), "{bad}") }
    }
}
