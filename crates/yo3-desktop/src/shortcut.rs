//! A desktop shortcut to the launcher, offered at the end of setup. On Windows a `.lnk` on the user's Desktop (made
//! through the Windows Script Host, the paths passed as environment variables, never as script text); on Linux a
//! desktop entry in the applications menu and, when the user has a Desktop folder, on it too.
use std::path::{Path, PathBuf};

/// Creates the shortcut for `exe` and returns where it went.
pub fn create(exe: &Path) -> anyhow::Result<Vec<PathBuf>> {
    #[cfg(windows)] {
        let script = "$d=[Environment]::GetFolderPath('Desktop'); $p=Join-Path $d 'YO3 Revival.lnk'; \
            $s=(New-Object -ComObject WScript.Shell).CreateShortcut($p); $s.TargetPath=$env:YO3_SHORTCUT_TARGET; \
            $s.WorkingDirectory=$env:YO3_SHORTCUT_DIR; $s.IconLocation=$env:YO3_SHORTCUT_TARGET+',0'; \
            $s.Description='YO3 Revival'; $s.Save(); Write-Output $p";
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
            .env("YO3_SHORTCUT_TARGET", exe).env("YO3_SHORTCUT_DIR", exe.parent().unwrap_or(exe));
        { use std::os::windows::process::CommandExt; cmd.creation_flags(0x08000000); }
        let out = cmd.output()?;
        anyhow::ensure!(out.status.success(), "Windows did not create the shortcut: {}", String::from_utf8_lossy(&out.stderr).trim());
        Ok(vec![PathBuf::from(String::from_utf8_lossy(&out.stdout).trim())])
    }
    #[cfg(not(windows))] {
        use std::os::unix::fs::PermissionsExt;
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"));
        let icon = exe.parent().map(|d| d.join("yo3-revival.png")).filter(|p| p.is_file());
        let entry = desktop_entry(exe, icon.as_deref());
        let mut places = vec![data.join("applications")];
        if let Some(desktop) = desktop_dir(&home) { places.push(desktop) }
        let mut written = Vec::new();
        for dir in places {
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("yo3-revival.desktop");
            std::fs::write(&path, &entry)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
            written.push(path);
        }
        Ok(written)
    }
}

/// Whether the shortcut [`create`] makes is there: on Windows the `.lnk` on the Desktop, on Linux the menu entry.
pub fn exists() -> bool {
    #[cfg(windows)] {
        use windows_sys::Win32::UI::Shell::{FOLDERID_Desktop, SHGetKnownFolderPath};
        let mut raw: windows_sys::core::PWSTR = std::ptr::null_mut();
        // SAFETY: the out pointer is ours; Windows allocates the path, freed below whatever the result.
        let ok = unsafe { SHGetKnownFolderPath(&FOLDERID_Desktop, 0, std::ptr::null_mut(), &mut raw) } == 0;
        let desktop = (ok && !raw.is_null()).then(|| {
            let len = (0..).take_while(|&i| unsafe { *raw.add(i) } != 0).count();
            PathBuf::from(String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(raw, len) }))
        });
        unsafe { windows_sys::Win32::System::Com::CoTaskMemFree(raw as _) };
        desktop.is_some_and(|d| d.join("YO3 Revival.lnk").is_file())
    }
    #[cfg(not(windows))] {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"));
        data.join("applications/yo3-revival.desktop").is_file()
    }
}

/// The user's Desktop folder, if they have one (`xdg-user-dir DESKTOP`, else `~/Desktop`).
#[cfg(not(windows))]
fn desktop_dir(home: &Path) -> Option<PathBuf> {
    let named = std::process::Command::new("xdg-user-dir").arg("DESKTOP").output().ok()
        .filter(|o| o.status.success()).map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()));
    named.filter(|d| d.is_dir() && d != home).or_else(|| Some(home.join("Desktop")).filter(|d| d.is_dir()))
}

/// A freedesktop.org desktop entry that starts `exe`.
pub fn desktop_entry(exe: &Path, icon: Option<&Path>) -> String {
    // Exec arguments are quoted, with `"`, `` ` ``, `$` and `\` escaped (Desktop Entry Specification, "Exec key");
    // the `\` doubling is the string-level escape applied on top.
    let quoted = exe.to_string_lossy().chars().fold(String::from("\""), |mut out, c| {
        if matches!(c, '"' | '`' | '$' | '\\') { out.push_str("\\\\") }
        if c == '%' { out.push('%') }
        out.push(c);
        out
    }) + "\"";
    let mut entry = format!("[Desktop Entry]\nType=Application\nName=YO3 Revival\nComment=Play Yu-Gi-Oh! ONLINE 3 again\nExec={quoted}\nTerminal=false\nCategories=Game;\n");
    if let Some(icon) = icon { entry.push_str(&format!("Icon={}\n", icon.display())) }
    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_entry_quotes_the_path() {
        let entry = desktop_entry(Path::new("/opt/my games/yo3 $x\"/yo3-revival"), None);
        assert!(entry.contains("Exec=\"/opt/my games/yo3 \\\\$x\\\\\"/yo3-revival\"\n"), "{entry}");
        assert!(entry.starts_with("[Desktop Entry]\nType=Application\nName=YO3 Revival\n"));
        assert!(!entry.contains("Icon="));
        assert!(desktop_entry(Path::new("/a/b"), Some(Path::new("/a/yo3-revival.png"))).contains("Icon=/a/yo3-revival.png\n"));
    }
}
