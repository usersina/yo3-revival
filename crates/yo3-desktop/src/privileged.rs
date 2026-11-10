//! Optional, narrow elevation. Ordinary server/game startup never calls this module's elevation path.
use std::path::Path;
use anyhow::{Context, ensure};

#[derive(Clone, Debug)]
pub struct CommandPlan { pub program: String, pub args: Vec<String>, pub env: Vec<(String, String)> }
#[derive(Clone, Copy)]
pub enum LinuxFirewall { Firewalld, Ufw }
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PermissionResult { Applied, Cancelled }

pub fn windows_firewall_plan(server: &Path, port: u16) -> anyhow::Result<CommandPlan> {
    ensure!(port > 0, "Choose a nonzero TCP port");
    // Paths are data in environment variables, never interpolated into PowerShell source.
    let script = "$ErrorActionPreference='Stop'; $name=$env:YO3_FW_RULE; $existing=Get-NetFirewallRule -Name $name -ErrorAction SilentlyContinue; if ($existing) { $app=$existing | Get-NetFirewallApplicationFilter; $port=$existing | Get-NetFirewallPortFilter; if ($existing.Action -ne 'Allow' -or $existing.Enabled -ne 'True' -or $existing.Profile.ToString() -ne 'Private' -or $app.Program -ne $env:YO3_FW_PROGRAM -or $port.LocalPort -ne $env:YO3_FW_PORT -or $port.Protocol -ne 'TCP') { throw 'The existing application firewall rule was modified. Review it in Windows Firewall.' }; return }; New-NetFirewallRule -Name $name -DisplayName ('YO3 Revival TCP '+$env:YO3_FW_PORT) -Direction Inbound -Action Allow -Program $env:YO3_FW_PROGRAM -Protocol TCP -LocalPort $env:YO3_FW_PORT -Profile Private | Out-Null";
    Ok(CommandPlan { program: "powershell.exe".into(), args: ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script].map(str::to_string).into(),
        env: vec![("YO3_FW_PROGRAM".into(), server.to_string_lossy().into_owned()), ("YO3_FW_PORT".into(), port.to_string()),
            ("YO3_FW_RULE".into(), format!("YO3Revival-TCP-{port}-{}", &yo3_launcher::components::hash(server.to_string_lossy().as_bytes())[..12]))] })
}
pub fn linux_firewall_plan(backend: LinuxFirewall, port: u16) -> anyhow::Result<Vec<CommandPlan>> {
    ensure!(port > 0, "Choose a nonzero TCP port");
    let plan = |program: &str, args: Vec<String>| CommandPlan { program: program.into(), args, env: vec![] };
    Ok(match backend {
        LinuxFirewall::Firewalld => vec![plan("firewall-cmd", vec!["--permanent".into(), format!("--add-port={port}/tcp")]), plan("firewall-cmd", vec![format!("--add-port={port}/tcp")])],
        LinuxFirewall::Ufw => vec![plan("ufw", vec!["allow".into(), format!("{port}/tcp"), "comment".into(), "YO3 Revival".into()])],
    })
}
fn execute(plan: CommandPlan) -> anyhow::Result<()> {
    let result = std::process::Command::new(&plan.program).args(&plan.args).envs(plan.env).output().with_context(|| format!("Run {}", plan.program))?;
    ensure!(result.status.success(), "{} refused firewall configuration: {}", plan.program, String::from_utf8_lossy(&result.stderr).trim());
    Ok(())
}
fn configure_firewall(port: u16) -> anyhow::Result<()> {
    ensure!(port > 0, "Choose a nonzero TCP port");
    #[cfg(windows)] {
        let exe = std::env::current_exe()?; let server = exe.parent().context("Application folder is missing")?.join("yo3-server.exe").canonicalize()?;
        let manifest = exe.parent().unwrap().join("bundle.json");
        if manifest.is_file() {
            let value: serde_json::Value = serde_json::from_slice(&std::fs::read(manifest)?)?;
            ensure!(value["components"]["server"]["sha256"].as_str() == Some(yo3_launcher::components::hash(&std::fs::read(&server)?).as_str()), "Server file does not match this release; repair before configuring its firewall rule");
        }
        execute(windows_firewall_plan(&server, port)?)
    }
    #[cfg(not(windows))] {
        let firewalld = std::process::Command::new("firewall-cmd").arg("--state").output().is_ok_and(|s| s.status.success());
        let ufw = std::process::Command::new("ufw").arg("status").env("LC_ALL", "C").output().is_ok_and(|s| s.status.success() && String::from_utf8_lossy(&s.stdout).starts_with("Status: active"));
        let backend = if firewalld { LinuxFirewall::Firewalld } else if ufw { LinuxFirewall::Ufw }
            else { anyhow::bail!("No running firewalld or active UFW was found. Allow TCP {port} using your firewall's own settings; the server can still run.") };
        for command in linux_firewall_plan(backend, port)? { execute(command)?; }
        Ok(())
    }
}

/// Recognized before Iced initializes, so an elevated helper never creates an elevated manager/game.
pub fn helper_mode() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    let mode = args.next();
    if mode.as_deref() == Some(std::ffi::OsStr::new("--loader-helper")) {
        let action = args.next(); let game = args.next();
        let result = match (action.as_deref().and_then(|v| v.to_str()), game) {
            (Some(action @ ("install" | "remove")), Some(game)) if args.next().is_none() => crate::direct::elevated_install(Path::new(&game), action == "remove"),
            _ => Err(anyhow::anyhow!("Expected install/remove and one game folder")),
        };
        return Some(match result { Ok(()) => 0, Err(error) => { eprintln!("{error:#}"); 30 } });
    }
    if mode.as_deref() != Some(std::ffi::OsStr::new("--firewall-helper")) { return None }
    let port = args.next().and_then(|s| s.to_str().and_then(|s| s.parse::<u16>().ok()));
    let result = match port { Some(port) if port > 0 && args.next().is_none() => configure_firewall(port), _ => Err(anyhow::anyhow!("Expected only a TCP port from 1 to 65535")) };
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

pub fn request_firewall(port: u16) -> anyhow::Result<PermissionResult> {
    ensure!(port > 0, "Choose a nonzero TCP port");
    #[cfg(not(windows))] {
        let output = std::process::Command::new("pkexec").arg(std::env::current_exe()?).args(["--firewall-helper", &port.to_string()]).output()
            .context("The permission helper pkexec is unavailable; configure your firewall manually")?;
        if output.status.code() == Some(126) { return Ok(PermissionResult::Cancelled) }
        ensure!(output.status.success(), "Firewall setup was not completed: {}", String::from_utf8_lossy(&output.stderr).trim());
        Ok(PermissionResult::Applied)
    }
    #[cfg(windows)] { elevated_windows(&format!("--firewall-helper {port}")) }
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
        if started == 0 { if error == ERROR_CANCELLED { return Ok(PermissionResult::Cancelled) } anyhow::bail!("Windows could not start firewall setup (error {error})") }
        if info.hProcess.is_null() { return Ok(PermissionResult::Cancelled) }
        let waited = WaitForSingleObject(info.hProcess, INFINITE); let mut code = 30; let read = GetExitCodeProcess(info.hProcess, &mut code); CloseHandle(info.hProcess);
        ensure!(waited == 0 && read != 0 && code == 0, "Windows firewall setup was not completed (helper code {code}). Check permissions/policy, or allow the server's TCP port manually.");
        Ok(PermissionResult::Applied)
    }
}
